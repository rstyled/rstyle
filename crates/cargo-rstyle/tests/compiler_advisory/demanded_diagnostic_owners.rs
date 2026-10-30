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

fn replay_positive_without_sources_or_tools(directory: &Path, original: &Value) {
    let bytes = std::fs::read(directory.join("baseline.json")).unwrap();
    std::fs::remove_dir_all(directory.join("src")).unwrap();
    std::fs::remove_file(directory.join("Cargo.toml")).unwrap();
    let (output, same) = replay(directory, original, COMPARE);
    checked(&output, 0);
    assert_eq!(baseline(&same)["coverage"], "complete");
    assert_eq!(baseline(&same)["existing_findings"], 1);
    assert_eq!(baseline(&same)["new_findings"], 0);
    assert_eq!(unit(&same)["evaluation"], unit(original)["evaluation"]);
    assert_eq!(
        std::fs::read(directory.join("baseline.json")).unwrap(),
        bytes
    );
}

fn clean_baseline(directory: &Path, clean: &Value) {
    let (output, report) = replay(directory, clean, COMPARE);
    checked(&output, 0);
    assert_eq!(baseline(&report)["coverage"], "complete");
    let retained = baseline(&report)["retained"].as_array().unwrap();
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0]["state"], "resolved");
}

fn changed_compilation_context(directory: &Path) {
    let (output, changed) = capture(directory, &[COMPARE, &["--release"]].concat());
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

fn prune_completed_review(directory: &Path, clean: &Value) {
    let (output, status) = replay(directory, clean, STATUS);
    checked(&output, 0);
    let records = status["review_status"]["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["state"], "no_finding");
    let bytes = std::fs::read(directory.join("reviews.json")).unwrap();
    let (output, dry) = replay(
        directory,
        clean,
        &[STATUS, &["--review-prune", "--review-prune-dry-run"]].concat(),
    );
    checked(&output, 0);
    assert_eq!(dry["review_prune"]["eligible_entries"], 1);
    assert_eq!(dry["review_prune"]["removed_entries"], 0);
    assert_eq!(
        std::fs::read(directory.join("reviews.json")).unwrap(),
        bytes
    );
    let (output, pruned) = replay(directory, clean, &[STATUS, &["--review-prune"]].concat());
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

fn missing_measurement_stays_incomplete(directory: &Path, clean: &Value) {
    let bundle = clean["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    let original_wire: Value = serde_json::from_slice(&std::fs::read(bundle).unwrap()).unwrap();
    let ledger = only::ledger_path(clean);
    let changed = missing_late_observation(clean);
    only::reseal_raw(clean, &ledger, &changed);
    let (output, missing) = replay(directory, clean, COMPARE);
    checked(&output, 2);
    assert_eq!(baseline(&missing)["coverage"], "incomplete");
    assert_eq!(unit(&missing)["diagnostic_scope"]["input_complete"], false);
    let retained = baseline(&missing)["retained"].as_array().unwrap();
    assert_eq!(retained.len(), 1);
    assert_ne!(retained[0]["state"], "resolved");
    only::reseal_raw(clean, &ledger, &original_wire);
}

#[test]
fn late_owner_baselines_and_reviews_preserve_clean_missing_and_context_outcomes() {
    let fixture = fixture();
    let directory = fixture.path();
    let original = exported(directory);
    dismiss_late(directory, &original);
    write_source(directory, false, false);
    let (output, clean) = capture(directory, &[]);
    checked(&output, 0);
    clean_baseline(directory, &clean);
    missing_measurement_stays_incomplete(directory, &clean);
    prune_completed_review(directory, &clean);
    changed_compilation_context(directory);
    replay_positive_without_sources_or_tools(directory, &original);
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
