use super::*;

const CLEAN: &str = "pub fn mixed()->u128 {0}";

fn reviewed_from_configured(directory: &TempDir, original: &Value) {
    for index in 0..diagnostics(original).len() {
        dismiss(directory.path(), original, index);
    }
}
fn current(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-advisory",
        "--compiler-advisory-config",
        "policy.toml",
        "--review-file",
        "reviews.json",
        "--review-status",
    ];
    args.extend_from_slice(extra);
    if extra.contains(&"--review-prune") && !extra.contains(&"--replay-run") {
        let capture: Vec<_> = args
            .iter()
            .copied()
            .filter(|arg| !["--review-prune", "--review-prune-dry-run"].contains(arg))
            .collect();
        let (_, captured) = clones(directory, &capture);
        args.retain(|arg| *arg != "--release");
        args.extend(["--replay-run", captured["run_directory"].as_str().unwrap()]);
        return clones(directory, &args);
    }
    clones(directory, &args)
}
fn records(report: &Value) -> &[Value] {
    report["review_status"]["records"].as_array().unwrap()
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
fn states(report: &Value, expected: &str) {
    assert_eq!(records(report).len(), 6);
    assert!(
        records(report).iter().all(|row| row["state"] == expected),
        "{}",
        report["review_status"]
    );
}

#[test]
fn completed_six_rule_negatives_support_readonly_status_dry_run_and_explicit_pruning() {
    let (directory, _, raw) = capture(SOURCE, &[]);
    for scenario in [
        suppressed_invalid_and_excluded_owners_cannot_retire_numeric_choices_scenario
            as fn(&TempDir, &Value),
        changed_policy_context_and_unselected_files_keep_retained_choices_scenario
            as fn(&TempDir, &Value),
        missing_and_generated_owners_do_not_become_numeric_negatives_scenario
            as fn(&TempDir, &Value),
        direct_negatives_survive_unrelated_generated_inputs_but_density_remains_unavailable_scenario
            as fn(&TempDir, &Value),
        completed_numeric_negatives_do_not_require_strict_pair_completion_scenario
            as fn(&TempDir, &Value),
        completed_six_rule_negatives_support_readonly_status_dry_run_and_explicit_pruning_scenario
            as fn(&TempDir, &Value),
    ] {
        reset_review_fixture(&directory);
        let original = configured_from_capture(&directory, &raw);
        reviewed_from_configured(&directory, &original);
        scenario(&directory, &original);
    }
}

fn completed_six_rule_negatives_support_readonly_status_dry_run_and_explicit_pruning_scenario(
    directory: &TempDir,
    _original: &Value,
) {
    let path = directory.path().join("reviews.json");
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), CLEAN).unwrap();
    let (output, clean) = current(directory.path(), &[]);
    checked(&output, 0);
    states(&clean, "no_finding");
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let (output, dry) = current(
        directory.path(),
        &["--review-prune", "--review-prune-dry-run"],
    );
    checked(&output, 0);
    assert_eq!(dry["review_prune"]["eligible_entries"], 6);
    assert_eq!(dry["review_prune"]["removed_entries"], 0);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let (output, pruned) = current(
        directory.path(),
        &[
            "--replay-run",
            clean["run_directory"].as_str().unwrap(),
            "--review-prune",
        ],
    );
    checked(&output, 0);
    assert_eq!(pruned["review_prune"]["removed_entries"], 6);
    let saved: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(saved["schema_version"], 9);
    assert!(saved.get("compiler_diagnostic_entries").is_none());
}

fn suppressed_invalid_and_excluded_owners_cannot_retire_numeric_choices_scenario(
    directory: &TempDir,
    _original: &Value,
) {
    let path = directory.path().join("reviews.json");
    let bytes = std::fs::read(&path).unwrap();
    for comment in [
        "// rstyle-allow(fn_length,fn_params,fn_spacing,mutation_per_target,mutated_targets,side_effect_density): fixture",
        "// rstyle-allow(unknown): invalid fixture",
    ] {
        std::fs::write(
            directory.path().join("src/lib.rs"),
            format!("{comment}\n{CLEAN}"),
        )
        .unwrap();
        let (output, report) = current(directory.path(), &["--review-prune"]);
        checked(&output, 2);
        states(&report, "unavailable");
        assert_eq!(report["review_prune"]["removed_entries"], 0);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    std::fs::write(directory.path().join("src/lib.rs"), CLEAN).unwrap();
    std::fs::write(
        directory.path().join("policy.toml"),
        format!("exclude=['src/**']\n{CONFIG}"),
    )
    .unwrap();
    let (output, excluded) = current(directory.path(), &["--review-prune"]);
    checked(&output, 2);
    states(&excluded, "unavailable");
    assert!(
        records(&excluded)[0]["detail"]
            .as_str()
            .unwrap()
            .contains("excluded")
    );
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

fn changed_policy_context_and_unselected_files_keep_retained_choices_scenario(
    directory: &TempDir,
    _original: &Value,
) {
    let path = directory.path().join("reviews.json");
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), CLEAN).unwrap();
    std::fs::write(
        directory.path().join("policy.toml"),
        CONFIG.replace("max_params=1", "max_params=8"),
    )
    .unwrap();
    let (output, policy) = current(directory.path(), &["--review-prune"]);
    checked(&output, 0);
    states(&policy, "stale");
    std::fs::write(directory.path().join("policy.toml"), CONFIG).unwrap();
    let (output, release) = current(directory.path(), &["--release", "--review-prune"]);
    checked(&output, 0);
    states(&release, "context_changed");
    std::fs::write(
        directory.path().join("selection.json"),
        "{\"schema\":\"rstyle.changed-files/1\",\"paths\":[]}",
    )
    .unwrap();
    let (output, unselected) = current(
        directory.path(),
        &["--changed-files", "selection.json", "--review-prune"],
    );
    checked(&output, 2);
    states(&unselected, "not_requested");
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

fn missing_and_generated_owners_do_not_become_numeric_negatives_scenario(
    directory: &TempDir,
    _original: &Value,
) {
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn different()->u128 {0}",
    )
    .unwrap();
    let (output, missing) = current(directory.path(), &["--review-prune"]);
    checked(&output, 2);
    states(&missing, "not_observed");
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "macro_rules! make {()=>{pub fn mixed()->u128 {0}}} make!();",
    )
    .unwrap();
    let (output, generated) = current(directory.path(), &["--review-prune"]);
    checked(&output, 2);
    states(&generated, "unavailable");
    assert_eq!(generated["review_prune"]["removed_entries"], 0);
}

fn direct_negatives_survive_unrelated_generated_inputs_but_density_remains_unavailable_scenario(
    directory: &TempDir,
    _original: &Value,
) {
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{CLEAN}\nmacro_rules! make {{()=>{{pub fn generated(){{}}}}}} make!();"),
    )
    .unwrap();
    let (output, report) = current(
        directory.path(),
        &["--review-prune", "--review-prune-dry-run"],
    );
    checked(&output, 2);
    assert_eq!(
        records(&report)
            .iter()
            .filter(|row| row["state"] == "no_finding")
            .count(),
        5
    );
    assert_eq!(
        records(&report)
            .iter()
            .filter(|row| row["state"] == "unavailable")
            .count(),
        1
    );
    assert_eq!(report["review_prune"]["eligible_entries"], 5);
    assert_eq!(report["compiler_advisory"]["complete"], false);
}

#[test]
fn baseline_resolution_reuses_the_same_completed_scope_gates() {
    let (directory, original) = baseline::exported();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{CLEAN}\nmacro_rules! make {{()=>{{pub fn generated(){{}}}}}} make!();"),
    )
    .unwrap();
    let (output, report) = baseline::capture_in(directory.path(), baseline::COMPARE);
    checked(&output, 2);
    let retained = report["compiler_advisory"]["diagnostic_baseline"]["retained"]
        .as_array()
        .unwrap();
    assert_eq!(
        retained
            .iter()
            .filter(|row| row["state"] == "resolved")
            .count(),
        5
    );
    assert_eq!(
        retained
            .iter()
            .filter(|row| row["state"] == "unavailable")
            .count(),
        1
    );
    assert_ne!(report["matching"], Value::Null);
    assert_ne!(original["matching"], Value::Null);
}

fn completed_numeric_negatives_do_not_require_strict_pair_completion_scenario(
    directory: &TempDir,
    _original: &Value,
) {
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{CLEAN} pub fn first(x:u8)->u8 {{x+1}} pub fn second(x:u8)->u8 {{x+2}}"),
    )
    .unwrap();
    let (output, report) = current(directory.path(), &["--max-pairs", "0"]);
    checked(&output, 2);
    assert_eq!(report["pair_evaluation_complete"], false);
    assert_eq!(report["compiler_advisory"]["complete"], true);
    states(&report, "no_finding");
}
