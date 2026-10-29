use super::propagation::checked;
use super::*;

const BASE: &[&str] = &[
    "--extraction-profile",
    "hir-data-v3",
    "--compiler-advisory-only",
    "--compiler-advisory",
    "--compiler-advisory-config",
    "policy.toml",
    "--compiler-diagnostic-rules",
    "fn_params",
];
const EXPORT: &[&str] = &["--write-compiler-baseline", "baseline.json"];
const COMPARE: &[&str] = &[
    "--compiler-baseline",
    "baseline.json",
    "--fail-on-new-compiler",
];
const STATUS: &[&str] = &["--review-file", "reviews.json", "--review-status"];
const PARAMS: &str = "_a:u32,_b:u32,_c:u32,_d:u32,_e:u32";

fn source(all_bad: bool, late_bad: bool) -> String {
    let mut source = String::new();
    for index in 0..4100 {
        let params = if all_bad { PARAMS } else { "" };
        source.push_str(&format!("pub fn f{index}({params}){{}}\n"));
    }
    let params = if late_bad { PARAMS } else { "" };
    source.push_str(&format!("pub fn late_bad({params}){{}}\n"));
    source
}

fn fixture() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("policy.toml"),
        "[fn_params]\nmax_params=4\n",
    )
    .unwrap();
    write_source(directory.path(), false, true);
    directory
}

fn write_source(directory: &Path, all_bad: bool, late_bad: bool) {
    std::fs::write(directory.join("src/lib.rs"), source(all_bad, late_bad)).unwrap();
}

fn capture(directory: &Path, extra: &[&str]) -> (Output, Value) {
    clones(directory, &[BASE, extra].concat())
}

fn replay_output(directory: &Path, original: &Value, extra: &[&str]) -> Output {
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
    diagnostic_process::output(&mut command).unwrap()
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let output = replay_output(directory, original, extra);
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)));
    (output, report)
}

fn unit(report: &Value) -> &Value {
    &report["compiler_advisory"]["units"][0]
}

fn baseline(report: &Value) -> &Value {
    &report["compiler_advisory"]["diagnostic_baseline"]
}

fn exported(directory: &Path) -> Value {
    let (output, report) = capture(directory, EXPORT);
    checked(&output, 0);
    assert_eq!(
        unit(&report)["observations"].as_array().unwrap().len(),
        4101
    );
    assert_eq!(baseline(&report)["coverage"], "complete");
    let stored: Value =
        serde_json::from_slice(&std::fs::read(directory.join("baseline.json")).unwrap()).unwrap();
    let entries = stored["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["rule_id"], "fn_params");
    assert_eq!(entries[0]["actual"], 5);
    assert_eq!(entries[0]["limit"], 4);
    report
}

#[test]
fn late_positive_exports_from_large_inventory_and_replays_without_sources_or_tools() {
    let directory = fixture();
    let original = exported(directory.path());
    let bytes = std::fs::read(directory.path().join("baseline.json")).unwrap();
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let (output, same) = replay(directory.path(), &original, COMPARE);
    checked(&output, 0);
    assert_eq!(baseline(&same)["coverage"], "complete");
    assert_eq!(baseline(&same)["existing_findings"], 1);
    assert_eq!(baseline(&same)["new_findings"], 0);
    assert_eq!(unit(&same)["evaluation"], unit(&original)["evaluation"]);
    assert_eq!(
        std::fs::read(directory.path().join("baseline.json")).unwrap(),
        bytes
    );
}

#[test]
fn retained_late_owner_resolves_clean_measurement_but_preserves_compilation_context_drift() {
    let directory = fixture();
    exported(directory.path());
    write_source(directory.path(), false, false);
    let (output, clean) = capture(directory.path(), COMPARE);
    checked(&output, 0);
    assert_eq!(baseline(&clean)["coverage"], "complete");
    let retained = baseline(&clean)["retained"].as_array().unwrap();
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0]["state"], "resolved");
    let (output, changed) = capture(directory.path(), &[COMPARE, &["--release"]].concat());
    checked(&output, 1);
    assert_eq!(baseline(&changed)["coverage"], "complete");
    assert_eq!(baseline(&changed)["scope_changed"], true);
    assert!(
        baseline(&changed)["retained"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["state"] == "context_changed")
    );
}

fn dismiss_late(directory: &Path, original: &Value) {
    let diagnostics = unit(original)["evaluation"]["diagnostics"]
        .as_array()
        .unwrap();
    let index = diagnostics
        .iter()
        .position(|row| row["rule_id"] == "fn_params")
        .unwrap();
    assert!(
        diagnostics[index]["message"]
            .as_str()
            .unwrap()
            .contains("late_bad")
    );
    let index = index.to_string();
    let (output, _) = replay(
        directory,
        original,
        &[
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-diagnostic",
            "0",
            &index,
            "--review-reason",
            "Reviewed late fixture owner",
        ],
    );
    checked(&output, 0);
}

#[test]
fn retained_review_demands_late_clean_owner_and_prunes_only_completed_negative() {
    let directory = fixture();
    let (output, original) = capture(directory.path(), &[]);
    checked(&output, 0);
    dismiss_late(directory.path(), &original);
    write_source(directory.path(), false, false);
    let (output, clean) = capture(directory.path(), STATUS);
    checked(&output, 0);
    let records = clean["review_status"]["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["state"], "no_finding");
    let bytes = std::fs::read(directory.path().join("reviews.json")).unwrap();
    let (output, dry) = replay(
        directory.path(),
        &clean,
        &[STATUS, &["--review-prune", "--review-prune-dry-run"]].concat(),
    );
    checked(&output, 0);
    assert_eq!(dry["review_prune"]["eligible_entries"], 1);
    assert_eq!(dry["review_prune"]["removed_entries"], 0);
    assert_eq!(
        std::fs::read(directory.path().join("reviews.json")).unwrap(),
        bytes
    );
    let (output, pruned) = replay(
        directory.path(),
        &clean,
        &[STATUS, &["--review-prune"]].concat(),
    );
    checked(&output, 0);
    assert_eq!(pruned["review_prune"]["removed_entries"], 1);
}

fn missing_late_observation(report: &Value) -> Value {
    let path = report["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    let mut wire: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let fact = wire["facts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|fact| fact["name"] == "late_bad")
        .unwrap();
    fact["build"]["frontend_options"]
        .as_object_mut()
        .unwrap()
        .remove(rstyle_core::advisory::KEY);
    wire
}

#[test]
fn missing_late_measurement_never_becomes_a_clean_retained_negative() {
    let directory = fixture();
    exported(directory.path());
    write_source(directory.path(), false, false);
    let (output, clean) = capture(directory.path(), &[]);
    checked(&output, 0);
    let wire = missing_late_observation(&clean);
    only::reseal_raw(&clean, &only::ledger_path(&clean), &wire);
    let (output, missing) = replay(directory.path(), &clean, COMPARE);
    checked(&output, 2);
    assert_eq!(baseline(&missing)["coverage"], "incomplete");
    assert_eq!(unit(&missing)["diagnostic_scope"]["input_complete"], false);
    let retained = baseline(&missing)["retained"].as_array().unwrap();
    assert_eq!(retained.len(), 1);
    assert_ne!(retained[0]["state"], "resolved");
}

#[test]
fn demanded_positive_payloads_keep_existing_4096_budget() {
    let directory = fixture();
    write_source(directory.path(), true, true);
    let (output, original) = capture(directory.path(), &[]);
    checked(&output, 0);
    assert_eq!(
        unit(&original)["diagnostic_scope"]["diagnostic_indices"]
            .as_array()
            .unwrap()
            .len(),
        4101
    );
    let output = replay_output(directory.path(), &original, EXPORT);
    checked(&output, 2);
    assert!(!directory.path().join("baseline.json").exists());
    assert!(!output.stderr.is_empty());
}
