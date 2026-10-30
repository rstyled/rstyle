use super::*;

const POSITIVE: &str = "pub fn mixed()->u128 {std::fs::write(\"/unused\",b\"x\").ok(); 0}";
const NEGATIVE: &str = "pub fn mixed()->u128 {0}";
const GENERATED: &str = "macro_rules! make {()=>{pub fn generated(){}}} make!();";
const POLICY: &str =
    "[side_effects]\nmax_density_milli=1\nmin_statements=0\nmin_lines=0\ntransitive_depth=0\n";
const STATUS: &[&str] = &[
    "--compiler-advisory-config",
    "policy.toml",
    "--review-file",
    "reviews.json",
    "--review-status",
];

fn checked(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}\n{}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn initial(depth: usize) -> (TempDir, Value) {
    let (directory, output, original) = capture(POSITIVE, &[]);
    checked(&output, 0);
    std::fs::write(
        directory.path().join("policy.toml"),
        POLICY.replace("transitive_depth=0", &format!("transitive_depth={depth}")),
    )
    .unwrap();
    let (output, report) = replay(
        directory.path(),
        &original,
        &["--compiler-advisory-config", "policy.toml"],
    );
    checked(&output, 0);
    assert_eq!(diagnostics(&report).len(), 1);
    assert_eq!(diagnostics(&report)[0]["rule_id"], "side_effect_density");
    dismiss(directory.path(), &report, 0);
    (directory, report)
}

fn current(directory: &Path, source: &str, extra: &[&str]) -> (Output, Value) {
    std::fs::write(directory.join("src/lib.rs"), source).unwrap();
    let mut args = STATUS.to_vec();
    args.extend_from_slice(extra);
    baseline::capture_in(directory, &args)
}

fn state(report: &Value, expected: &str) {
    let records = report["review_status"]["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["state"], expected, "{records:?}");
}

#[test]
fn direct_density_negative_retires_only_its_review_while_generated_source_is_missing() {
    let (directory, _) = initial(0);
    let path = directory.path().join("reviews.json");
    let bytes = std::fs::read(&path).unwrap();
    let (output, clean) = current(
        directory.path(),
        &format!("pub fn mixed()->u128 {{generated(); 0}}\n{GENERATED}"),
        &[],
    );
    checked(&output, 2);
    state(&clean, "no_finding");
    assert_eq!(clean["compiler_advisory"]["complete"], false);
    assert_eq!(
        clean["compiler_advisory"]["units"][0]["unresolved_callees"],
        serde_json::json!([])
    );
    assert_eq!(
        clean["compiler_advisory"]["units"][0]["unavailable"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(diagnostics(&clean).is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let mut args = STATUS.to_vec();
    args.extend(["--review-prune", "--review-prune-dry-run"]);
    let (output, dry) = replay(directory.path(), &clean, &args);
    checked(&output, 2);
    state(&dry, "no_finding");
    assert_eq!(dry["review_prune"]["eligible_entries"], 1);
    assert_eq!(dry["review_prune"]["removed_entries"], 0);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    for source in ["src/lib.rs", "Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.path().join(source)).unwrap();
    }
    args.pop();
    let (output, pruned) = replay(directory.path(), &clean, &args);
    checked(&output, 2);
    assert_eq!(pruned["review_prune"]["removed_entries"], 1);
    assert_eq!(pruned["compiler_advisory"]["complete"], false);
    assert_eq!(pruned["matching"], clean["matching"]);
    let store: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert!(store.get("compiler_diagnostic_entries").is_none());
}

#[test]
fn direct_density_baseline_resolves_completed_owner_but_cannot_export_missing_scope() {
    let (directory, original) = initial(0);
    let (output, _) = replay(
        directory.path(),
        &original,
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--write-compiler-baseline",
            "baseline.json",
        ],
    );
    checked(&output, 0);
    let (output, clean) = current(
        directory.path(),
        &format!("{NEGATIVE}\n{GENERATED}"),
        &[
            "--compiler-baseline",
            "baseline.json",
            "--fail-on-new-compiler",
        ],
    );
    checked(&output, 2);
    let baseline = &clean["compiler_advisory"]["diagnostic_baseline"];
    assert_eq!(baseline["coverage"], "incomplete");
    let retained = baseline["retained"].as_array().unwrap();
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0]["state"], "resolved");
    assert_eq!(baseline["new_findings"], 0);
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--replay-run",
            clean["run_directory"].as_str().unwrap(),
            "--compiler-advisory-config",
            "policy.toml",
            "--write-compiler-baseline",
            "incomplete.json",
        ],
    );
    checked(&output, 2);
    assert!(!directory.path().join("incomplete.json").exists());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Incomplete analysis"));
}

#[test]
fn inherited_density_and_missing_queried_owner_still_cannot_retire_a_review() {
    for (depth, source, expected) in [
        (1, format!("{NEGATIVE}\n{GENERATED}"), "unavailable"),
        (0, "pub fn different()->u128 {0}".into(), "not_observed"),
    ] {
        let (directory, _) = initial(depth);
        let path = directory.path().join("reviews.json");
        let bytes = std::fs::read(&path).unwrap();
        let (output, report) = current(directory.path(), &source, &[]);
        checked(&output, 2);
        state(&report, expected);
        let mut args = STATUS.to_vec();
        args.push("--review-prune");
        let (output, pruned) = replay(directory.path(), &report, &args);
        checked(&output, 2);
        assert_eq!(pruned["review_prune"]["removed_entries"], 0);
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn direct_density_negatives_keep_suppression_exclusion_selection_and_policy_gates() {
    let (directory, _) = initial(0);
    let path = directory.path().join("reviews.json");
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(
        directory.path().join("selection.json"),
        "{\"schema\":\"rstyle.changed-files/1\",\"paths\":[]}",
    )
    .unwrap();
    for (comment, policy, extra, expected) in [
        (
            "// rstyle-allow(side_effect_density): reviewed fixture\n",
            POLICY.to_string(),
            vec![],
            "unavailable",
        ),
        (
            "",
            format!("exclude=['src/**']\n{POLICY}"),
            vec![],
            "unavailable",
        ),
        (
            "",
            POLICY.to_string(),
            vec!["--changed-files", "selection.json"],
            "not_requested",
        ),
        (
            "",
            POLICY.replace("max_density_milli=1", "max_density_milli=2"),
            vec![],
            "stale",
        ),
    ] {
        std::fs::write(directory.path().join("policy.toml"), policy).unwrap();
        let (output, report) = current(
            directory.path(),
            &format!("{comment}{NEGATIVE}\n{GENERATED}"),
            &extra,
        );
        checked(&output, 2);
        state(&report, expected);
        let mut args = STATUS.to_vec();
        args.extend(extra);
        args.push("--review-prune");
        let (output, pruned) = replay(directory.path(), &report, &args);
        checked(&output, 2);
        assert_eq!(pruned["review_prune"]["removed_entries"], 0);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn direct_density_negatives_preserve_context_and_metric_boundaries() {
    let (directory, _) = initial(0);
    let path = directory.path().join("reviews.json");
    let bytes = std::fs::read(&path).unwrap();
    let source = format!("{NEGATIVE}\n{GENERATED}");
    let (output, changed) = current(directory.path(), &source, &["--release"]);
    checked(&output, 2);
    // With incomplete scope, a missing exact context remains unavailable.
    state(&changed, "unavailable");
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    let mut store: Value = serde_json::from_slice(&bytes).unwrap();
    store["compiler_diagnostic_entries"][0]["finding"]["metrics"] =
        "rstyle.compiler-style-metrics/1".into();
    let bytes = serde_json::to_vec(&store).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let (output, changed) = current(directory.path(), &source, &[]);
    checked(&output, 2);
    state(&changed, "unavailable");
    let mut args = STATUS.to_vec();
    args.push("--review-prune");
    let (output, retained) = replay(directory.path(), &changed, &args);
    checked(&output, 2);
    assert_eq!(retained["review_prune"]["removed_entries"], 0);
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
