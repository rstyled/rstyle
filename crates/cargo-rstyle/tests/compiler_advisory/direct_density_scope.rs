use super::propagation::checked;
use super::*;

const CALLER: &str = "#[derive(Debug)] pub struct Generated(pub u32);\npub fn direct(){\nstd::fs::write(\"/unused\",b\"x\").ok();\nrenamed::leaf();\n}\npub fn pure(value:u128)->u128 {value}\n";
const BASE: &[&str] = &[
    "--extraction-profile",
    "hir-data-v3",
    "--compiler-advisory-only",
    "--compiler-advisory",
    "--compiler-advisory-config",
    "policy.toml",
];
const DIRECT: &[&str] = &[
    "--compiler-diagnostic-scope",
    "authored",
    "--compiler-diagnostic-rules",
    "side_effect_density",
];

fn policy(directory: &Path, depth: usize, limit: usize) {
    std::fs::write(directory.join("policy.toml"), format!(
        "[side_effects]\nmax_density_milli={limit}\nmin_statements=0\nmin_lines=0\ntransitive_depth={depth}\n"
    )).unwrap();
}

fn fixture() -> (TempDir, Value) {
    let directory =
        propagation::workspace("pub fn leaf(){std::fs::write(\"/unused\",b\"leaf\").ok();}");
    std::fs::write(directory.path().join("caller/src/lib.rs"), CALLER).unwrap();
    policy(directory.path(), 0, 500);
    let mut flags = BASE.to_vec();
    flags.extend(["--package", "caller"]);
    let (output, report) = clones(directory.path(), &flags);
    checked(&output, 2);
    assert!(!unit(&report)["unavailable"].as_array().unwrap().is_empty());
    assert_eq!(
        unit(&report)["unresolved_callees"],
        serde_json::json!(["leaf::leaf"])
    );
    (directory, report)
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .arg("clones")
        .args(BASE)
        .args([
            "--format",
            "json",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .args(extra)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    let output = diagnostic_process::output(&mut command).unwrap();
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)));
    (output, report)
}

fn unit(report: &Value) -> &Value {
    &report["compiler_advisory"]["units"][0]
}
fn scope(report: &Value) -> &Value {
    &unit(report)["diagnostic_scope"]
}
fn diagnostics(report: &Value) -> &Vec<Value> {
    unit(report)["evaluation"]["diagnostics"]
        .as_array()
        .unwrap()
}

fn unchanged(before: &Value, after: &Value) {
    for field in [
        "evaluation",
        "observations",
        "unavailable",
        "generated_effects",
        "unresolved_callees",
    ] {
        assert_eq!(unit(before)[field], unit(after)[field], "{field}");
    }
}

fn complete_direct(report: &Value) {
    assert_eq!(scope(report)["input_complete"], true);
    assert_eq!(scope(report)["effect_inputs_complete"], false);
    assert_eq!(scope(report)["unavailable_owners"], 0);
    assert_eq!(report["compiler_advisory"]["complete"], true);
    assert_eq!(report["matching_execution"], "not_requested");
    assert_eq!(
        scope(report)["request"]["rules"],
        serde_json::json!(["side_effect_density"])
    );
}

#[test]
fn authored_depth_zero_density_completes_and_baselines_replay_without_sources_or_tools() {
    let (directory, original) = fixture();
    assert!(unit(&original).get("diagnostic_scope").is_none());
    let (output, captured) = replay(
        directory.path(),
        &original,
        &[
            "--compiler-diagnostic-rules",
            "side_effect_density",
            "--compiler-diagnostic-scope",
            "captured",
        ],
    );
    checked(&output, 2);
    assert_eq!(scope(&captured)["input_complete"], false);
    let (output, selected) = replay(
        directory.path(),
        &original,
        &[DIRECT, &["--write-compiler-baseline", "baseline.json"]].concat(),
    );
    checked(&output, 0);
    complete_direct(&selected);
    unchanged(&original, &selected);
    let baseline = &selected["compiler_advisory"]["diagnostic_baseline"];
    assert_eq!(baseline["coverage"], "complete");
    baseline_without_sources(directory.path(), &original);
}

fn baseline_without_sources(directory: &Path, original: &Value) {
    let bytes = std::fs::read(directory.join("baseline.json")).unwrap();
    for path in ["caller", "leaf"] {
        std::fs::remove_dir_all(directory.join(path)).unwrap();
    }
    std::fs::remove_file(directory.join("Cargo.toml")).unwrap();
    let flags = [
        DIRECT,
        &[
            "--compiler-baseline",
            "baseline.json",
            "--fail-on-new-compiler",
        ],
    ]
    .concat();
    let (output, replayed) = replay(directory, original, &flags);
    checked(&output, 0);
    complete_direct(&replayed);
    let baseline = &replayed["compiler_advisory"]["diagnostic_baseline"];
    assert_eq!(baseline["coverage"], "complete");
    assert_eq!(baseline["existing_findings"], 1);
    assert_eq!(baseline["new_findings"], 0);
    assert_eq!(
        std::fs::read(directory.join("baseline.json")).unwrap(),
        bytes
    );
}

#[test]
fn direct_io_density_preserves_exact_core_measurements_and_inclusive_threshold() {
    let (directory, original) = fixture();
    let (output, positive) = replay(
        directory.path(),
        &original,
        &[DIRECT, &["--fail-on-findings"]].concat(),
    );
    checked(&output, 1);
    complete_direct(&positive);
    unchanged(&original, &positive);
    exact_direct_measurements(&positive);
    policy(directory.path(), 0, 1500);
    let (output, full) = replay(directory.path(), &original, &[]);
    checked(&output, 2);
    let (output, clean) = replay(
        directory.path(),
        &original,
        &[DIRECT, &["--fail-on-findings"]].concat(),
    );
    checked(&output, 0);
    complete_direct(&clean);
    unchanged(&full, &clean);
    assert_eq!(clean["compiler_advisory"]["findings"], 0);
    assert_eq!(
        unit(&clean)["evaluation"]["summary"]["max_density_milli"],
        1500
    );
}

fn exact_direct_measurements(positive: &Value) {
    let density: Vec<_> = diagnostics(positive)
        .iter()
        .filter(|row| row["rule_id"] == "side_effect_density")
        .collect();
    assert_eq!(density.len(), 1);
    assert_eq!(density[0]["actual"], 1500);
    assert_eq!(density[0]["limit"], 500);
    assert!(
        density[0]["details"]
            .as_array()
            .unwrap()
            .iter()
            .any(|detail| detail["count"] == 1
                && detail["label"].as_str().unwrap().contains("std::fs::write"))
    );
    assert_eq!(
        unit(positive)["evaluation"]["summary"]["effect_counts"]["io"],
        1
    );
    assert_eq!(
        scope(positive)["diagnostic_indices"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn inherited_density_and_planning_keep_their_missing_effect_routes_at_every_depth() {
    let (directory, original) = fixture();
    for depth in [1, 3] {
        policy(directory.path(), depth, 500);
        let (output, full) = replay(directory.path(), &original, &[]);
        checked(&output, 2);
        let (output, inherited) = replay(directory.path(), &original, DIRECT);
        checked(&output, 2);
        assert_eq!(scope(&inherited)["input_complete"], false);
        assert_eq!(scope(&inherited)["effect_inputs_complete"], false);
        unchanged(&full, &inherited);
    }
    policy(directory.path(), 0, 500);
    let (output, planned) = replay(
        directory.path(),
        &original,
        &[DIRECT, &["--compiler-plans"]].concat(),
    );
    checked(&output, 0);
    assert_eq!(scope(&planned)["input_complete"], true);
    assert_eq!(scope(&planned)["effect_inputs_complete"], false);
    assert_eq!(
        planned["compiler_advisory"]["planning"]["input_complete"],
        false
    );
    assert_eq!(planned["compiler_advisory"]["complete"], true);
    unchanged(&original, &planned);
}

fn raw_wire(report: &Value) -> Value {
    let path = report["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn damaged(wire: &Value, kind: &str) -> Value {
    let mut changed = wire.clone();
    for fact in changed["facts"].as_array_mut().unwrap() {
        if fact["name"] == "direct" && kind == "observation" {
            fact["build"]["frontend_options"]
                .as_object_mut()
                .unwrap()
                .remove(rstyle_core::advisory::KEY);
        }
        if kind == "origin"
            && fact["source_diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["construct"] == "macro_generated_function")
        {
            fact["source_diagnostics"]
                .as_array_mut()
                .unwrap()
                .retain(|row| row["construct"] != "macro_generated_function");
            let options = fact["build"]["frontend_options"].as_object_mut().unwrap();
            options.remove(rstyle_core::advisory::generated_effects::KEY);
            options.remove(rstyle_core::advisory::generated_effects::CONNECTED_KEY);
        }
    }
    changed
}

#[test]
fn direct_density_never_hides_missing_applicable_facts_or_unknown_generated_origins() {
    let (directory, original) = fixture();
    let wire = raw_wire(&original);
    let ledger = only::ledger_path(&original);
    for kind in ["observation", "origin"] {
        only::reseal_raw(&original, &ledger, &damaged(&wire, kind));
        let (output, incomplete) = replay(directory.path(), &original, DIRECT);
        checked(&output, 2);
        assert_eq!(scope(&incomplete)["input_complete"], false, "{kind}");
        assert_eq!(scope(&incomplete)["unavailable_owners"], 1);
        assert_eq!(scope(&incomplete)["effect_inputs_complete"], false);
        if kind == "origin" {
            assert!(
                scope(&incomplete)["source_scope"]["outside_scope_functions"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
    }
    only::reseal_raw(&original, &ledger, &wire);
    checked(&replay(directory.path(), &original, DIRECT).0, 0);
}
