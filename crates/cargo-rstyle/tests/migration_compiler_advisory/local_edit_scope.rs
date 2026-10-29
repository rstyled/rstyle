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
    "--compiler-plans",
    "--compiler-advisory-config",
    "policy.toml",
];
const DENSITY: &[&str] = &[
    "--compiler-diagnostic-scope",
    "authored",
    "--compiler-diagnostic-rules",
    "side_effect_density",
];

fn policy(directory: &Path, limit: usize, grouping: &str) {
    std::fs::write(directory.join("policy.toml"), format!(
        "[side_effects]\nmax_density_milli={limit}\nmin_statements=0\nmin_lines=0\ntransitive_depth=3\n{grouping}"
    )).unwrap();
}

fn fixture() -> (TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), CHAIN).unwrap();
    policy(directory.path(), 1, "");
    let (output, report) = clones(directory.path(), BASE);
    checked(&output, 2);
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

fn planning(report: &Value) -> &Value {
    &report["compiler_advisory"]["planning"]
}

fn required(report: &Value, complete: bool) {
    assert_eq!(planning(report)["required_edits_complete"], complete);
    assert_eq!(planning(report)["input_complete"], false);
    assert_eq!(planning(report)["known_functions"], 1);
}

fn unchanged(full: &Value, selected: &Value) {
    for field in [
        "evaluation",
        "observations",
        "unavailable",
        "generated_effects",
        "unresolved_callees",
        "propagation_complete",
    ] {
        assert_eq!(unit(full)[field], unit(selected)[field], "{field}");
    }
    for field in [
        "schema",
        "input_complete",
        "known_functions",
        "limitations",
        "groups",
    ] {
        assert_eq!(planning(full)[field], planning(selected)[field], "{field}");
    }
}

fn density_group(report: &Value) {
    let groups = planning(report)["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    let key = &unit(report)["observations"][0]["key"];
    assert_eq!(groups[0]["members"], serde_json::json!([key]));
    assert_eq!(groups[0]["step_owners"], serde_json::json!([]));
    let functions = groups[0]["evaluation"]["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 1);
    assert_eq!(functions[0]["name"], "authored_chain");
    assert_eq!(
        functions[0]["rules"],
        serde_json::json!(["side_effect_density"])
    );
    assert!(groups[0]["evaluation"].get("plan").is_none());
}

#[test]
fn body_local_density_edits_complete_without_claiming_full_generated_planning_inputs() {
    let (directory, original) = fixture();
    let (output, selected) = replay(directory.path(), &original, DENSITY);
    checked(&output, 0);
    required(&selected, true);
    density_group(&selected);
    unchanged(&original, &selected);
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let (output, saved) = replay(directory.path(), &original, DENSITY);
    checked(&output, 0);
    required(&saved, true);
    assert_eq!(saved["compiler_advisory"], selected["compiler_advisory"]);
}

#[test]
fn density_caller_footprints_require_missing_generated_inputs_only_at_positive_hops() {
    let (directory, original) = fixture();
    for (hops, complete) in [(1, false), (0, true)] {
        policy(
            directory.path(),
            1,
            &format!("[grouping]\ncaller_rules=['side_effect_density']\ncaller_hops={hops}\n"),
        );
        let (output, full) = replay(directory.path(), &original, &[]);
        checked(&output, 2);
        let (output, selected) = replay(directory.path(), &original, DENSITY);
        checked(&output, 0);
        required(&selected, complete);
        density_group(&selected);
        unchanged(&full, &selected);
    }
}

#[test]
fn signature_plan_still_needs_generated_callsite_census_with_zero_grouping_hops() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn target(a:u32,b:u32,c:u32,d:u32,e:u32){let _=(a,b,c,d,e);}\n\
         macro_rules! make {()=>{pub fn generated_caller(){target(1,2,3,4,5);}}} make!();\n",
    )
    .unwrap();
    policy(directory.path(), 1, "[grouping]\ncaller_hops=0\n");
    let (output, report) = clones(
        directory.path(),
        &[
            BASE,
            &[
                "--compiler-diagnostic-scope",
                "authored",
                "--compiler-diagnostic-rules",
                "fn_params",
            ],
        ]
        .concat(),
    );
    checked(&output, 0);
    required(&report, false);
    let groups = planning(&report)["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(
        groups[0]["evaluation"]["functions"][0]["rules"],
        serde_json::json!(["fn_params"])
    );
    assert!(
        groups[0]["evaluation"]["plan"]["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| step["signature"] == true)
    );
}

fn raw_wire(report: &Value) -> Value {
    let path = report["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn damaged(wire: &Value, kind: &str) -> Value {
    let mut changed = wire.clone();
    let target = if kind == "measurement" {
        "authored_chain"
    } else {
        "generated_leaf"
    };
    let fact = changed["facts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|fact| fact["name"] == target)
        .unwrap();
    let options = fact["build"]["frontend_options"].as_object_mut().unwrap();
    let key = if kind == "measurement" {
        rstyle_core::advisory::KEY
    } else {
        rstyle_core::advisory::generated_effects::CONNECTED_KEY
    };
    assert!(options.remove(key).is_some());
    changed
}

#[test]
fn required_edits_remain_incomplete_when_selected_measurements_or_effect_routes_are_missing() {
    let (directory, original) = fixture();
    let wire = raw_wire(&original);
    let ledger = only::ledger_path(&original);
    for kind in ["measurement", "effects"] {
        only::reseal_raw(&original, &ledger, &damaged(&wire, kind));
        let (output, incomplete) = replay(directory.path(), &original, DENSITY);
        checked(&output, 2);
        assert_eq!(
            planning(&incomplete)["required_edits_complete"],
            false,
            "{kind}"
        );
        assert_eq!(
            unit(&incomplete)["diagnostic_scope"]["input_complete"],
            false
        );
    }
    only::reseal_raw(&original, &ledger, &wire);
    let (output, restored) = replay(directory.path(), &original, DENSITY);
    checked(&output, 0);
    required(&restored, true);
}

#[test]
fn empty_selected_worklist_completes_only_with_complete_selected_diagnostics() {
    let (directory, original) = fixture();
    policy(directory.path(), 3000, "");
    let (output, clean) = replay(directory.path(), &original, DENSITY);
    checked(&output, 0);
    required(&clean, true);
    assert_eq!(planning(&clean)["groups"], serde_json::json!([]));
    let wire = damaged(&raw_wire(&original), "effects");
    only::reseal_raw(&original, &only::ledger_path(&original), &wire);
    let (output, missing) = replay(directory.path(), &original, DENSITY);
    checked(&output, 2);
    required(&missing, false);
    assert_eq!(planning(&missing)["groups"], serde_json::json!([]));
}
