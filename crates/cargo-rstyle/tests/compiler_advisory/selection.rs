use super::*;

const POLICY: &str = "[fn_length]\nmax_lines=100\n[fn_params]\nmax_params=1\n";
const BAD: &str = "pub fn many(a:u128,b:u128)->u128 {a+b}";

fn workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::create_dir_all(directory.path().join("src/nested")).unwrap();
    for (path, source) in [
        (
            "src/lib.rs",
            "pub mod clean; pub mod bad; pub mod nested {pub mod bad;}",
        ),
        ("src/clean.rs", "pub fn clean(value:u128)->u128 {value}"),
        ("src/bad.rs", BAD),
        ("src/nested/bad.rs", BAD),
        ("policy.toml", POLICY),
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

fn requested(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-advisory",
        "--compiler-advisory-config",
        "policy.toml",
    ];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut args = vec!["--replay-run", original["run_directory"].as_str().unwrap()];
    args.extend_from_slice(extra);
    requested(directory, &args)
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
fn scope(report: &Value) -> &Value {
    &report["compiler_advisory"]["changed_scope"]
}
fn diagnostics(report: &Value) -> &Value {
    &report["compiler_advisory"]["units"][0]["evaluation"]
}
const FILTER: &[&str] = &["--changed-files", "selection.json", "--fail-on-findings"];

#[test]
fn compiler_file_selection_preserves_raw_reports_and_exact_path_boundaries() {
    let directory = workspace();
    let (output, original) = requested(directory.path(), &[]);
    checked(&output, 0);
    assert_eq!(original["compiler_advisory"]["findings"], 2);
    let (output, clean) = replay(directory.path(), &original, FILTER);
    checked(&output, 0);
    assert_eq!(scope(&clean)["visible_findings"], 0);
    assert_eq!(scope(&clean)["coverage"], "complete");
    assert_eq!(clean["analysis"]["execution"], "not_requested");
    select(directory.path(), &["src/bad.rs"]);
    let (output, bad) = replay(directory.path(), &original, FILTER);
    checked(&output, 1);
    assert_eq!(scope(&bad)["visible_findings"], 1);
    assert_eq!(scope(&bad)["paths"][0]["matched_functions"], 1);
    assert_eq!(diagnostics(&bad), diagnostics(&original));
    assert_eq!(bad["matching"], original["matching"]);
    assert_eq!(bad["compiler_advisory"]["findings"], 2);
    assert!(bad["compiler_advisory"]["diagnostic_review"].is_null());
    assert!(!directory.path().join("reviews.json").exists());
}

#[test]
fn compiler_file_selection_filters_text_diagnostics() {
    let directory = workspace();
    let (_, original) = requested(directory.path(), &[]);
    select(directory.path(), &["src/bad.rs"]);
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--format",
            "text",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-advisory-config",
            "policy.toml",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--changed-files",
            "selection.json",
            "--fail-on-findings",
        ],
    );
    checked(&output, 1);
    let text = stdout(&output);
    let diagnostics: Vec<_> = text
        .lines()
        .filter(|line| line.starts_with("Compiler diagnostic unit"))
        .collect();
    assert_eq!(diagnostics.len(), 1, "{text}");
    assert!(diagnostics[0].contains("src/bad.rs:"), "{text}");
    assert!(!diagnostics[0].contains("src/nested/bad.rs:"), "{text}");
    assert!(text.contains("Changed-file compiler diagnostics: Complete coverage"));
}

#[test]
fn empty_selection_and_unobserved_files_have_distinct_outcomes() {
    let directory = workspace();
    let (_, original) = requested(directory.path(), &[]);
    select(directory.path(), &[]);
    let (output, empty) = replay(directory.path(), &original, FILTER);
    checked(&output, 0);
    assert_eq!(scope(&empty)["paths"], serde_json::json!([]));
    select(directory.path(), &["src/missing.rs"]);
    let (output, missing) = replay(directory.path(), &original, FILTER);
    checked(&output, 2);
    assert_eq!(scope(&missing)["paths"][0]["status"], "unobserved");
    assert_eq!(scope(&missing)["coverage"], "incomplete");
}

#[test]
fn compiler_selection_replays_without_sources_or_compiler_tools() {
    let directory = workspace();
    let (_, original) = requested(directory.path(), &[]);
    select(directory.path(), &["src/bad.rs"]);
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory.path())
        .args([
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-advisory-config",
            "policy.toml",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--changed-files",
            "selection.json",
            "--fail-on-findings",
        ])
        .env("PATH", directory.path().join("no-tools"));
    let output = diagnostic_process::output(&mut command).unwrap();
    checked(&output, 1);
    let saved: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(scope(&saved)["visible_findings"], 1);
    assert_eq!(diagnostics(&saved), diagnostics(&original));
}

#[test]
fn generated_or_missing_observations_cannot_be_hidden_by_clean_file_selection() {
    let directory = workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub mod clean; macro_rules! make {()=>{pub fn generated(){}}} make!();",
    )
    .unwrap();
    let (output, generated) = requested(directory.path(), FILTER);
    checked(&output, 2);
    assert_eq!(generated["compiler_advisory"]["complete"], false);
    std::fs::write(directory.path().join("src/lib.rs"), "pub mod clean;").unwrap();
    let (_, original) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    let (output, missing) = replay(directory.path(), &original, FILTER);
    checked(&output, 2);
    assert_eq!(scope(&missing)["paths"][0]["unavailable_functions"], 1);
    assert_eq!(scope(&missing)["coverage"], "incomplete");
}

#[test]
fn compiler_file_selection_keeps_check_and_test_occurrences_separate() {
    let directory = workspace();
    select(directory.path(), &["src/bad.rs"]);
    let (output, report) = requested(
        directory.path(),
        &["--all-targets", "--changed-files", "selection.json"],
    );
    checked(&output, 2);
    assert_eq!(scope(&report)["paths"][0]["matched_functions"], 2);
    assert_eq!(scope(&report)["visible_findings"], 2);
    let units = report["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    assert!(
        units
            .iter()
            .all(|unit| unit["selected_diagnostics"].as_array().unwrap().len() == 1)
    );
}

#[test]
fn selected_reviews_hide_only_the_completed_visible_diagnostics() {
    let directory = workspace();
    let (_, original) = requested(directory.path(), &[]);
    let index = diagnostics(&original)["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .position(|diagnostic| diagnostic["path"] == "src/bad.rs")
        .unwrap()
        .to_string();
    let (output, _) = replay(
        directory.path(),
        &original,
        &[
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-diagnostic",
            "0",
            &index,
            "--review-reason",
            "Reviewed fixture parameter budget",
        ],
    );
    checked(&output, 0);
    select(directory.path(), &["src/bad.rs"]);
    let (output, reviewed) = replay(
        directory.path(),
        &original,
        &[
            "--review-file",
            "reviews.json",
            "--changed-files",
            "selection.json",
            "--fail-on-findings",
        ],
    );
    checked(&output, 0);
    assert_eq!(scope(&reviewed)["visible_findings"], 0);
    assert_eq!(reviewed["compiler_advisory"]["findings"], 1);
    assert_eq!(diagnostics(&reviewed), diagnostics(&original));
}

#[test]
fn compiler_baseline_counts_selected_changes_but_keeps_whole_scope_drift() {
    let directory = workspace();
    let (output, _) = requested(
        directory.path(),
        &["--write-compiler-baseline", "baseline.json"],
    );
    checked(&output, 0);
    std::fs::write(
        directory.path().join("src/bad.rs"),
        BAD.replace("b:u128)", "b:u128,c:u128)"),
    )
    .unwrap();
    let args = [
        "--compiler-baseline",
        "baseline.json",
        "--changed-files",
        "selection.json",
        "--fail-on-new-compiler",
    ];
    let (output, clean) = requested(directory.path(), &args);
    checked(&output, 0);
    assert_eq!(
        clean["compiler_advisory"]["diagnostic_baseline"]["new_findings"],
        0
    );
    select(directory.path(), &["src/bad.rs"]);
    let (output, changed) = requested(directory.path(), &args);
    checked(&output, 1);
    assert_eq!(
        changed["compiler_advisory"]["diagnostic_baseline"]["new_findings"],
        1
    );
    select(directory.path(), &[]);
    std::fs::write(
        directory.path().join("policy.toml"),
        POLICY.replace("max_params=1", "max_params=0"),
    )
    .unwrap();
    let (output, drift) = requested(directory.path(), &args);
    checked(&output, 1);
    assert_eq!(
        drift["compiler_advisory"]["diagnostic_baseline"]["scope_changed"],
        true
    );
}

#[test]
fn invalid_suppression_diagnostics_remain_global_when_files_are_filtered() {
    let directory = workspace();
    std::fs::write(
        directory.path().join("src/bad.rs"),
        format!("// rstyle-allow(unknown): invalid fixture\n{BAD}"),
    )
    .unwrap();
    let (output, report) = requested(directory.path(), FILTER);
    checked(&output, 1);
    assert_eq!(scope(&report)["visible_findings"], 0);
    let unit = &report["compiler_advisory"]["units"][0];
    let selected = unit["selected_diagnostics"].as_array().unwrap();
    assert_eq!(selected.len(), 1);
    let index = selected[0].as_u64().unwrap() as usize;
    assert_eq!(
        unit["evaluation"]["diagnostics"][index]["rule_id"],
        "invalid_suppression"
    );
}

#[test]
fn compiler_proposals_remain_visible_outside_diagnostic_file_selection() {
    let directory = workspace();
    std::fs::write(
        directory.path().join("src/bad.rs"),
        include_str!("../fixtures/animals/src/lib.rs"),
    )
    .unwrap();
    let (output, report) = requested(
        directory.path(),
        &[
            "--compiler-patterns",
            "--changed-files",
            "selection.json",
            "--fail-on-findings",
        ],
    );
    checked(&output, 1);
    assert_eq!(scope(&report)["visible_findings"], 0);
    let units = report["compiler_advisory"]["units"].as_array().unwrap();
    assert!(units.iter().any(|unit| {
        !unit["patterns"]["evaluation"]["candidates"]
            .as_array()
            .unwrap()
            .is_empty()
    }));
}
