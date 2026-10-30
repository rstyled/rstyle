use super::propagation::checked;
use super::*;

const CHAIN: &str = r#"
macro_rules! make {()=>{
    pub fn generated_leaf() {let _ = std::fs::read("/unused");}
    pub fn generated_middle() {generated_leaf();}
}}
make!();
pub fn authored_chain() {generated_middle();}
"#;
const BASE: &[&str] = &[
    "--extraction-profile",
    "hir-data-v3",
    "--compiler-advisory-only",
    "--compiler-advisory",
    "--compiler-advisory-config",
    "policy.toml",
];
const AUTHORED: &[&str] = &[
    "--compiler-diagnostic-scope",
    "authored",
    "--compiler-diagnostic-rules",
    "side_effect_density",
];

fn policy(directory: &Path, depth: usize) {
    std::fs::write(directory.join("policy.toml"), format!(
        "[side_effects]\nmax_density_milli=1\nmin_statements=0\nmin_lines=0\ntransitive_depth={depth}\n"
    )).unwrap();
}

fn fixture() -> (TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), CHAIN).unwrap();
    policy(directory.path(), 3);
    let (output, report) = clones(directory.path(), BASE);
    checked(&output, 2);
    assert_eq!(unit(&report)["unavailable"].as_array().unwrap().len(), 2);
    assert_eq!(
        unit(&report)["generated_effects"].as_array().unwrap().len(),
        2
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

fn same_inputs_and_evaluation(full: &Value, scoped: &Value) {
    for field in [
        "evaluation",
        "observations",
        "unavailable",
        "generated_effects",
        "unresolved_callees",
        "propagation_complete",
    ] {
        assert_eq!(unit(full)[field], unit(scoped)[field], "{field}");
    }
}

fn complete(report: &Value) {
    assert_eq!(scope(report)["input_complete"], true);
    assert_eq!(scope(report)["effect_inputs_complete"], true);
    assert_eq!(scope(report)["unavailable_owners"], 0);
    assert_eq!(report["compiler_advisory"]["complete"], true);
    assert_eq!(report["matching_execution"], "not_requested");
}

fn measured_density(report: &Value, expected: usize) {
    assert_eq!(
        unit(report)["evaluation"]["summary"]["effect_counts"]["io"]
            .as_u64()
            .unwrap_or(0),
        expected as u64
    );
    let rows: Vec<_> = unit(report)["evaluation"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["rule_id"] == "side_effect_density")
        .collect();
    assert_eq!(rows.len(), expected);
    assert_eq!(
        scope(report)["diagnostic_indices"]
            .as_array()
            .unwrap()
            .len(),
        expected
    );
    if expected == 1 {
        assert_eq!(rows[0]["actual"], 3000);
        assert_eq!(rows[0]["limit"], 1);
        assert!(rows[0]["details"].as_array().unwrap().iter().any(|detail| {
            detail["count"] == 1 && detail["label"].as_str().unwrap().contains("std::fs::read")
        }));
    }
}

#[test]
fn connected_generated_chain_completes_authored_density_at_existing_propagation_depths() {
    let (directory, original) = fixture();
    assert!(unit(&original).get("diagnostic_scope").is_none());
    for (depth, expected) in [(1, 0), (2, 1), (3, 1)] {
        policy(directory.path(), depth);
        let (output, full) = replay(directory.path(), &original, &[]);
        checked(&output, 2);
        let (output, selected) = replay(directory.path(), &original, AUTHORED);
        checked(&output, 0);
        complete(&selected);
        measured_density(&selected, expected);
        same_inputs_and_evaluation(&full, &selected);
        let (output, captured) = replay(
            directory.path(),
            &original,
            &["--compiler-diagnostic-rules", "side_effect_density"],
        );
        checked(&output, 2);
        assert_eq!(scope(&captured)["input_complete"], false);
    }
}

#[test]
fn inherited_authored_density_baseline_replays_without_sources_or_tools() {
    let (directory, original) = fixture();
    let (output, selected) = replay(
        directory.path(),
        &original,
        &[AUTHORED, &["--write-compiler-baseline", "baseline.json"]].concat(),
    );
    checked(&output, 0);
    complete(&selected);
    assert_eq!(
        selected["compiler_advisory"]["diagnostic_baseline"]["coverage"],
        "complete"
    );
    let bytes = std::fs::read(directory.path().join("baseline.json")).unwrap();
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let (output, replayed) = replay(
        directory.path(),
        &original,
        &[
            AUTHORED,
            &[
                "--compiler-baseline",
                "baseline.json",
                "--fail-on-new-compiler",
            ],
        ]
        .concat(),
    );
    checked(&output, 0);
    complete(&replayed);
    same_inputs_and_evaluation(&selected, &replayed);
    let baseline = &replayed["compiler_advisory"]["diagnostic_baseline"];
    assert_eq!(baseline["coverage"], "complete");
    assert_eq!(baseline["existing_findings"], 1);
    assert_eq!(baseline["new_findings"], 0);
    assert_eq!(
        std::fs::read(directory.path().join("baseline.json")).unwrap(),
        bytes
    );
}

#[test]
fn connected_generated_effects_do_not_supply_missing_numeric_planning_facts() {
    let (directory, original) = fixture();
    let (output, planned) = replay(
        directory.path(),
        &original,
        &[AUTHORED, &["--compiler-plans"]].concat(),
    );
    checked(&output, 0);
    assert_eq!(scope(&planned)["input_complete"], true);
    assert_eq!(scope(&planned)["effect_inputs_complete"], true);
    assert_eq!(
        planned["compiler_advisory"]["planning"]["input_complete"],
        false
    );
    assert_eq!(planned["compiler_advisory"]["complete"], true);
    same_inputs_and_evaluation(&original, &planned);
}

fn raw_wire(report: &Value) -> Value {
    let path = report["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn damage_generated(fact: &mut Value, kind: &str) {
    use rstyle_core::advisory::generated_effects::{CONNECTED_KEY, KEY};
    let options = fact["build"]["frontend_options"].as_object_mut().unwrap();
    let row = options.remove(CONNECTED_KEY).unwrap();
    if kind == "historical" {
        let mut row = row;
        row["metrics"] = KEY.into();
        row.as_object_mut().unwrap().remove("connections");
        options.insert(KEY.into(), row);
    } else if kind == "origin" {
        fact["source_diagnostics"]
            .as_array_mut()
            .unwrap()
            .retain(|row| row["construct"] != "macro_generated_function");
    }
}

fn damaged(wire: &Value, kind: &str) -> Value {
    let mut changed = wire.clone();
    let facts = changed["facts"].as_array_mut().unwrap();
    let target = if kind == "authored" {
        "authored_chain"
    } else {
        "generated_leaf"
    };
    let fact = facts
        .iter_mut()
        .find(|fact| fact["name"] == target)
        .unwrap();
    if kind == "authored" {
        fact["build"]["frontend_options"]
            .as_object_mut()
            .unwrap()
            .remove(rstyle_core::advisory::KEY);
    } else {
        damage_generated(fact, kind);
    }
    changed
}

#[test]
fn inherited_density_requires_connected_rows_known_origins_and_applicable_authored_facts() {
    let (directory, original) = fixture();
    let wire = raw_wire(&original);
    let ledger = only::ledger_path(&original);
    for kind in ["missing", "historical", "authored", "origin"] {
        only::reseal_raw(&original, &ledger, &damaged(&wire, kind));
        let (output, incomplete) = replay(directory.path(), &original, AUTHORED);
        checked(&output, 2);
        assert_eq!(scope(&incomplete)["input_complete"], false, "{kind}");
        assert_eq!(
            scope(&incomplete)["effect_inputs_complete"],
            false,
            "{kind}"
        );
        assert_eq!(incomplete["compiler_advisory"]["complete"], false, "{kind}");
    }
    only::reseal_raw(&original, &ledger, &wire);
    let (output, restored) = replay(directory.path(), &original, AUTHORED);
    checked(&output, 0);
    complete(&restored);
}

#[test]
fn authored_inherited_density_keeps_unresolved_dependency_routes_incomplete() {
    let directory = propagation::workspace(
        "macro_rules! make {()=>{pub fn leaf(){let _=std::fs::read(\"/unused\");}}} make!();",
    );
    policy(directory.path(), 3);
    let (output, original) = clones(directory.path(), &[BASE, &["-p", "caller"]].concat());
    checked(&output, 2);
    let (output, selected) = replay(directory.path(), &original, AUTHORED);
    checked(&output, 2);
    assert_eq!(scope(&selected)["input_complete"], false);
    assert_eq!(scope(&selected)["effect_inputs_complete"], false);
    assert_eq!(
        unit(&selected)["unresolved_callees"],
        serde_json::json!(["leaf::leaf"])
    );
    same_inputs_and_evaluation(&original, &selected);
}

#[test]
fn selected_generated_dependency_supplies_existing_connected_effect_route() {
    let directory = propagation::workspace(
        "macro_rules! make {()=>{pub fn leaf(){let _=std::fs::read(\"/unused\");}}} make!();",
    );
    policy(directory.path(), 3);
    let (output, selected) = clones(directory.path(), &[BASE, AUTHORED].concat());
    checked(&output, 0);
    assert_eq!(selected["compiler_advisory"]["complete"], true);
    let caller = propagation::unit(&selected, "caller");
    let leaf = propagation::unit(&selected, "leaf");
    for unit in [caller, leaf] {
        assert_eq!(unit["diagnostic_scope"]["input_complete"], true);
        assert_eq!(unit["diagnostic_scope"]["effect_inputs_complete"], true);
        assert_eq!(unit["unresolved_callees"], serde_json::json!([]));
    }
    assert_eq!(caller["evaluation"]["summary"]["effect_counts"]["io"], 1);
    assert_eq!(
        caller["resolved_dependency_callees"]["leaf::leaf"],
        leaf["unit"]
    );
    assert_eq!(leaf["observations"], serde_json::json!([]));
    assert_eq!(leaf["generated_effects"].as_array().unwrap().len(), 1);
    assert_eq!(leaf["unavailable"].as_array().unwrap().len(), 1);
}
