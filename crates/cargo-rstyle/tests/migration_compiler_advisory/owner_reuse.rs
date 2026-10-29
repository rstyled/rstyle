use super::propagation::{capture, checked, workspace};
use super::*;

fn live(directory: &Path, extra: &[&str], code: i32) -> Value {
    let mut args = vec!["--compiler-advisory-only"];
    args.extend_from_slice(extra);
    let (output, report) = capture(directory, &args);
    checked(&output, code);
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["handoff_failures"], serde_json::json!([]));
    report
}

fn run(report: &Value) -> &Path {
    Path::new(report["run_directory"].as_str().unwrap())
}

fn index(report: &Value) -> std::path::PathBuf {
    run(report).with_file_name("advisory-owners.json")
}

fn fresh(report: &Value, name: &str, expected: bool) {
    let text = std::fs::read_to_string(run(report).join("cargo.jsonl")).unwrap();
    let records: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|row| row["reason"] == "compiler-artifact" && row["target"]["name"] == name)
        .collect();
    assert!(!records.is_empty(), "missing {name}: {text}");
    assert!(
        records.iter().all(|row| row["fresh"] == expected),
        "{records:?}"
    );
}

fn owned(report: &Value, reused: bool) {
    let functions = report["functions"].as_array().unwrap();
    assert!(!functions.is_empty());
    for function in functions {
        let bundle = Path::new(function["unit_evidence"]["bundle"].as_str().unwrap());
        assert!(bundle.starts_with(run(report)), "{}", bundle.display());
        assert!(bundle.is_file(), "{}", bundle.display());
        if reused {
            assert!(
                bundle
                    .parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("invocation-reused-")
            );
        }
    }
}

#[test]
fn completed_advisory_owners_reuse_current_fresh_artifacts_in_new_run_evidence() {
    let directory = clone_workspace();
    let first = live(directory.path(), &[], 0);
    let second = live(directory.path(), &[], 0);
    assert_ne!(first["run_directory"], second["run_directory"]);
    assert_eq!(first["compiler_advisory"], second["compiler_advisory"]);
    fresh(&first, "clone_fixture", false);
    fresh(&second, "clone_fixture", true);
    owned(&first, false);
    owned(&second, true);
    assert!(!run(&second).join("cargo-clean.stderr").exists());
}

#[test]
fn reused_advisory_owners_reevaluate_consumer_thresholds_without_compilation() {
    let directory = clone_workspace();
    let original = live(directory.path(), &[], 0);
    std::fs::write(
        directory.path().join("policy.toml"),
        "[fn_params]\nmax_params=0\n",
    )
    .unwrap();
    let changed = live(
        directory.path(),
        &["--compiler-advisory-config", "policy.toml"],
        0,
    );
    fresh(&changed, "clone_fixture", true);
    owned(&changed, true);
    let before = &original["compiler_advisory"]["units"][0];
    let after = &changed["compiler_advisory"]["units"][0];
    assert_eq!(before["observations"], after["observations"]);
    assert_ne!(
        before["evaluation"]["diagnostics"],
        after["evaluation"]["diagnostics"]
    );
    assert!(
        after["evaluation"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["rule_id"] == "fn_params")
    );
}

#[test]
fn source_edits_replace_completed_advisory_owner_observations() {
    let directory = clone_workspace();
    let _first = live(directory.path(), &[], 0);
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn changed() {}\n").unwrap();
    let changed = live(directory.path(), &[], 0);
    fresh(&changed, "clone_fixture", false);
    let observations = changed["compiler_advisory"]["units"][0]["observations"]
        .as_array()
        .unwrap();
    assert_eq!(observations.len(), 1);
    assert!(
        observations[0]["name"]
            .as_str()
            .unwrap()
            .ends_with("changed")
    );
    let again = live(directory.path(), &[], 0);
    fresh(&again, "clone_fixture", true);
    assert_eq!(changed["compiler_advisory"], again["compiler_advisory"]);
}

#[test]
fn missing_corrupt_and_incomplete_owner_indexes_force_recapture() {
    let directory = clone_workspace();
    let mut previous = live(directory.path(), &[], 0);
    for replacement in [None, Some("{"), Some("{\"schema\":1}")] {
        let path = index(&previous);
        match replacement {
            None => std::fs::remove_file(path).unwrap(),
            Some(text) => std::fs::write(path, text).unwrap(),
        }
        let current = live(directory.path(), &[], 0);
        fresh(&current, "clone_fixture", false);
        assert_eq!(previous["compiler_advisory"], current["compiler_advisory"]);
        previous = current;
    }
}

#[test]
fn missing_completed_snapshot_and_changed_tool_stamp_force_recapture() {
    let directory = clone_workspace();
    let first = live(directory.path(), &[], 0);
    let path = index(&first);
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let snapshot = Path::new(saved["entries"][0]["unit"]["directory"].as_str().unwrap());
    std::fs::remove_file(snapshot.join("unit.evidence.json")).unwrap();
    let restored = live(directory.path(), &[], 0);
    fresh(&restored, "clone_fixture", false);
    assert_eq!(first["compiler_advisory"], restored["compiler_advisory"]);
    saved = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let length = saved["tools"][0]["len"].as_u64().unwrap();
    saved["tools"][0]["len"] = (length + 1).into();
    std::fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let restamped = live(directory.path(), &[], 0);
    fresh(&restamped, "clone_fixture", false);
    assert_eq!(
        restored["compiler_advisory"],
        restamped["compiler_advisory"]
    );
}

#[test]
fn collector_changes_invalidate_completed_owner_reuse() {
    let directory = clone_workspace();
    let first = live(directory.path(), &[], 0);
    let patterns = live(directory.path(), &["--compiler-patterns"], 0);
    fresh(&patterns, "clone_fixture", false);
    let patterns_again = live(directory.path(), &["--compiler-patterns"], 0);
    fresh(&patterns_again, "clone_fixture", true);
    let ordinary = live(directory.path(), &[], 0);
    fresh(&ordinary, "clone_fixture", false);
    assert_eq!(first["compiler_advisory"], ordinary["compiler_advisory"]);
}

#[test]
fn partial_failed_build_discards_old_owner_index_before_successful_sibling_compiles() {
    let directory = workspace("pub fn leaf() {}\n");
    let first = live(directory.path(), &[], 0);
    std::fs::write(
        directory.path().join("leaf/src/lib.rs"),
        "pub fn leaf() {} pub fn added() {}\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        "pub fn broken() { this_does_not_exist(); }\n",
    )
    .unwrap();
    let (output, failed) = capture(directory.path(), &["--compiler-advisory-only"]);
    checked(&output, 2);
    assert_eq!(failed["capture_complete"], false);
    fresh(&failed, "leaf", false);
    assert!(!index(&first).exists());
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        "pub fn caller() { renamed::leaf(); }\n",
    )
    .unwrap();
    let recovered = live(directory.path(), &[], 0);
    fresh(&recovered, "leaf", false);
    fresh(&recovered, "caller", false);
    let leaf = super::propagation::unit(&recovered, "leaf");
    assert!(
        leaf["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["name"].as_str().unwrap().ends_with("added"))
    );
}

fn rustflags(directory: &Path, value: &str) {
    std::fs::create_dir_all(directory.join(".cargo")).unwrap();
    std::fs::write(
        directory.join(".cargo/config.toml"),
        format!("[build]\nrustflags=['--cfg','{value}']\n"),
    )
    .unwrap();
}

fn runs(report: &Value) -> usize {
    std::fs::read_dir(run(report).parent().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_str().unwrap().starts_with("run-"))
        .count()
}

#[test]
fn alternating_rustflags_recover_in_a_new_attempt_without_duplicate_ledgers() {
    let directory = clone_workspace();
    rustflags(directory.path(), "owner_a");
    let first = live(directory.path(), &[], 0);
    rustflags(directory.path(), "owner_b");
    let second = live(directory.path(), &[], 0);
    let before = runs(&second);
    rustflags(directory.path(), "owner_a");
    let third = live(directory.path(), &[], 0);
    assert_eq!(
        runs(&third),
        before + 2,
        "old fresh Cargo artifact requires a separate retry namespace"
    );
    fresh(&third, "clone_fixture", false);
    owned(&third, false);
    assert_eq!(first["compiler_advisory"], third["compiler_advisory"]);
    let fourth = live(directory.path(), &[], 0);
    fresh(&fourth, "clone_fixture", true);
    owned(&fourth, true);
}

#[test]
fn saved_reused_run_replays_without_original_run_sources_or_shared_builds() {
    let directory = clone_workspace();
    let first = live(directory.path(), &[], 0);
    let reused = live(directory.path(), &[], 0);
    fresh(&reused, "clone_fixture", true);
    owned(&reused, true);
    std::fs::remove_dir_all(run(&first)).unwrap();
    for path in ["src", "target/rstyle/clones/advisory-build"] {
        std::fs::remove_dir_all(directory.path().join(path)).unwrap();
    }
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    std::fs::remove_file(index(&reused)).unwrap();
    let (output, replay) =
        super::reviews::replay(directory.path(), &reused, &["--compiler-advisory-only"]);
    checked(&output, 0);
    assert_eq!(replay["capture_complete"], true);
    assert_eq!(reused["compiler_advisory"], replay["compiler_advisory"]);
}
