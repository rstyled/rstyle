use super::*;

const FILTER: &[&str] = &[
    "--select-compiler-patterns",
    "--changed-files",
    "selection.json",
    "--fail-on-findings",
];

fn workspace() -> TempDir {
    let directory = clone_workspace();
    let (definitions, usage) = ANIMALS.split_once("pub fn emit_dogs").unwrap();
    for (path, source) in [
        (
            "src/lib.rs",
            "pub mod definitions; pub mod usage; pub mod clean;".into(),
        ),
        ("src/definitions.rs", definitions.into()),
        (
            "src/usage.rs",
            format!("use crate::definitions::*;\npub fn emit_dogs{usage}"),
        ),
        (
            "src/clean.rs",
            "pub fn identity(value:u128)->u128 {value}".into(),
        ),
    ] {
        std::fs::write(directory.path().join(path), source).unwrap();
    }
    select(directory.path(), &["src/clean.rs"]);
    directory
}

fn select(directory: &Path, paths: &[&str]) {
    std::fs::write(
        directory.join("selection.json"),
        serde_json::to_vec(&serde_json::json!({
            "schema":"rstyle.changed-files/1", "paths":paths,
        }))
        .unwrap(),
    )
    .unwrap();
}

fn current(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-advisory",
        "--compiler-patterns",
        "--compiler-plans",
    ];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut args = vec!["--compiler-plans"];
    args.extend_from_slice(extra);
    super::replay(directory, original, &args)
}

fn checked(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}\n{}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn unchanged(original: &Value, selected: &Value) {
    for field in ["evaluation", "observations"] {
        assert_eq!(
            original["compiler_advisory"]["units"][0][field],
            selected["compiler_advisory"]["units"][0][field],
            "{field}"
        );
    }
    assert_eq!(
        proposals(original)["evaluation"],
        proposals(selected)["evaluation"]
    );
    assert_eq!(
        original["compiler_advisory"]["planning"],
        selected["compiler_advisory"]["planning"]
    );
    assert_eq!(original["matching"], selected["matching"]);
    assert_eq!(original["accepted_pairs"], selected["accepted_pairs"]);
}

#[test]
fn compiler_pattern_file_selection_is_opt_in_and_preserves_global_evaluations() {
    let directory = workspace();
    let (output, original) = current(directory.path(), FILTER);
    checked(&output, 0);
    assert!(
        !proposals(&original)["evaluation"]["candidates"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        proposals(&original)["selected_candidates"],
        serde_json::json!([])
    );
    assert_eq!(
        original["compiler_advisory"]["changed_scope"]["visible_findings"],
        0
    );
    let (output, legacy) = replay(
        directory.path(),
        &original,
        &["--changed-files", "selection.json", "--fail-on-findings"],
    );
    checked(&output, 1);
    assert!(proposals(&legacy)["selected_candidates"].is_null());
    unchanged(&original, &legacy);
    select(directory.path(), &["src/usage.rs"]);
    let (output, selected) = replay(directory.path(), &original, FILTER);
    checked(&output, 1);
    let count = proposals(&original)["evaluation"]["candidates"]
        .as_array()
        .unwrap()
        .len();
    assert_eq!(
        proposals(&selected)["selected_candidates"]
            .as_array()
            .unwrap()
            .len(),
        count
    );
    unchanged(&original, &selected);
}

#[test]
fn compiler_pattern_definition_paths_select_trait_proposals_without_usage_paths() {
    let directory = workspace();
    let (_, original) = current(directory.path(), &[]);
    select(directory.path(), &["src/definitions.rs"]);
    let (output, selected) = replay(directory.path(), &original, FILTER);
    checked(&output, 1);
    let candidates = proposals(&selected)["evaluation"]["candidates"]
        .as_array()
        .unwrap();
    let index = candidates
        .iter()
        .position(|row| row["kind"] == "trait_method")
        .unwrap();
    assert!(
        candidates[index]["sites"]
            .as_array()
            .unwrap()
            .iter()
            .all(|site| site["path"] == "src/usage.rs")
    );
    assert!(
        candidates[index]["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|site| site["path"] == "src/definitions.rs")
    );
    assert!(
        proposals(&selected)["selected_candidates"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(index))
    );
    unchanged(&original, &selected);
}

#[test]
fn compiler_pattern_empty_and_unobserved_file_selections_remain_distinct() {
    let directory = workspace();
    let (_, original) = current(directory.path(), &[]);
    for (paths, code) in [
        (vec![], 0),
        (vec!["src/nested/usage.rs"], 2),
        (vec!["src/usage.rs/child.rs"], 2),
        (vec!["src/missing.rs"], 2),
    ] {
        select(directory.path(), &paths);
        let (output, selected) = replay(directory.path(), &original, FILTER);
        checked(&output, code);
        assert_eq!(
            proposals(&selected)["selected_candidates"],
            serde_json::json!([])
        );
        if !paths.is_empty() {
            assert_eq!(
                selected["compiler_advisory"]["changed_scope"]["coverage"],
                "incomplete"
            );
        }
        unchanged(&original, &selected);
    }
}

#[test]
fn compiler_pattern_file_selection_requires_patterns_and_changed_files() {
    let directory = workspace();
    for extra in [
        vec!["--compiler-patterns"],
        vec!["--changed-files", "selection.json"],
    ] {
        let mut args = vec![
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--select-compiler-patterns",
        ];
        args.extend(extra);
        checked(&rstyle(directory.path(), &args), 2);
    }
    assert!(!directory.path().join("target/rstyle/clones").exists());
}

#[test]
fn compiler_pattern_selected_replay_needs_no_sources_and_filters_text() {
    let directory = workspace();
    let (_, original) = current(directory.path(), &[]);
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory.path())
        .args([
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-patterns",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .args(FILTER)
        .env("PATH", directory.path().join("no-tools"));
    let output = diagnostic_process::output(&mut command).unwrap();
    checked(&output, 0);
    assert!(!stdout(&output).contains("emit_dogs"));
    select(directory.path(), &["src/usage.rs"]);
    let output = diagnostic_process::output(&mut command).unwrap();
    checked(&output, 1);
    assert!(stdout(&output).contains("emit_dogs"));
}

#[test]
fn compiler_pattern_selection_and_review_masks_are_independent() {
    let helpers = "pub fn first(a:u128)->u128 {std::hint::black_box(a.wrapping_add(7)).wrapping_mul(3)} pub fn second(a:u128)->u128 {std::hint::black_box(a.wrapping_add(8)).wrapping_mul(3)}";
    let (directory, _, original) = capture(helpers, &["--compiler-patterns", "--compiler-plans"]);
    let candidate = &proposals(&original)["evaluation"]["candidates"][0];
    reviews::dismiss_candidate(directory.path(), &original, candidate, 0);
    select(directory.path(), &["src/lib.rs"]);
    let mut args = FILTER.to_vec();
    args.extend(["--review-file", "reviews.json"]);
    let (output, selected) = replay(directory.path(), &original, &args);
    checked(&output, 0);
    assert_eq!(
        proposals(&selected)["selected_candidates"],
        serde_json::json!([0])
    );
    assert_eq!(
        proposals(&selected)["visible_candidates"],
        serde_json::json!([])
    );
    unchanged(&original, &selected);
    select(directory.path(), &[]);
    let (output, empty) = replay(directory.path(), &original, &args);
    checked(&output, 0);
    assert_eq!(
        proposals(&empty)["selected_candidates"],
        serde_json::json!([])
    );
}

#[test]
fn compiler_pattern_selection_keeps_check_test_occurrences_and_incomplete_graphs() {
    let directory = workspace();
    select(directory.path(), &["src/usage.rs"]);
    let mut args = FILTER.to_vec();
    args.push("--all-targets");
    let (output, report) = current(directory.path(), &args);
    checked(&output, 2);
    let units = report["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    assert_ne!(units[0]["unit"], units[1]["unit"]);
    assert!(units.iter().all(|unit| {
        !unit["patterns"]["selected_candidates"]
            .as_array()
            .unwrap()
            .is_empty()
    }));
    select(directory.path(), &[]);
    let (output, empty) = replay(directory.path(), &report, FILTER);
    checked(&output, 2);
    assert_eq!(empty["compiler_advisory"]["complete"], false);
    let (directory, _, missing) = capture(GCD, &[]);
    select(directory.path(), &[]);
    let (output, report) = replay(directory.path(), &missing, FILTER);
    checked(&output, 2);
    assert_eq!(proposals(&report)["input_complete"], false);
    assert_eq!(
        proposals(&report)["selected_candidates"],
        serde_json::json!([])
    );
}

#[test]
fn compiler_pattern_baseline_counts_selected_changes_but_keeps_raw_changes_and_drift() {
    let directory = workspace();
    let (output, original) = current(
        directory.path(),
        &["--write-compiler-pattern-baseline", "baseline.json"],
    );
    checked(&output, 0);
    let path = directory.path().join("baseline.json");
    let mut baseline: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    baseline["entries"] = serde_json::json!([]);
    std::fs::write(path, serde_json::to_vec(&baseline).unwrap()).unwrap();
    let args = [
        "--select-compiler-patterns",
        "--changed-files",
        "selection.json",
        "--compiler-pattern-baseline",
        "baseline.json",
        "--fail-on-new-compiler-patterns",
    ];
    let (output, clean) = replay(directory.path(), &original, &args);
    checked(&output, 0);
    assert_eq!(
        clean["compiler_advisory"]["pattern_baseline"]["new_findings"],
        0
    );
    select(directory.path(), &["src/usage.rs"]);
    let (output, selected) = replay(directory.path(), &original, &args);
    checked(&output, 1);
    let before = &clean["compiler_advisory"]["pattern_baseline"];
    let after = &selected["compiler_advisory"]["pattern_baseline"];
    let mut unselected = before["current"].clone();
    for row in unselected.as_array_mut().unwrap() {
        assert_eq!(row["selected"], false);
        row["selected"] = serde_json::json!(true);
    }
    assert_eq!(unselected, after["current"]);
    assert!(after["new_findings"].as_u64().unwrap() > 0);
    select(directory.path(), &[]);
    std::fs::write(
        directory.path().join("policy.toml"),
        "[fn_length]\nmax_lines=99\n",
    )
    .unwrap();
    let mut drift_args = args.to_vec();
    drift_args.extend(["--compiler-advisory-config", "policy.toml"]);
    let (output, drift) = replay(directory.path(), &original, &drift_args);
    checked(&output, 1);
    assert_eq!(
        drift["compiler_advisory"]["pattern_baseline"]["scope_changed"],
        true
    );
}
