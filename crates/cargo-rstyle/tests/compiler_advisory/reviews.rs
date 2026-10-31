use super::*;

const SOURCE: &str = "pub fn mixed(a:u128,b:u128)->u128 {\nlet mut x=a;\nlet mut y=b;\nx+=1;\nx+=2;\ny+=3;\nstd::fs::write(\"/unused\",b\"x\").ok();\nx+y\n}";
const CONFIG: &str = "[fn_length]\nmax_lines=2\n[fn_params]\nmax_params=1\n[fn_spacing]\nmin_statements=2\n[mutation_budget]\nmax_mutations_per_target=1\nmax_mutated_targets=1\n[side_effects]\nmax_density_milli=1\nmin_statements=0\nmin_lines=0\n";

fn configured() -> (TempDir, Value) {
    let (directory, _output, original) = capture(SOURCE, &[]);
    let configured = configured_from_capture(&directory, &original);
    (directory, configured)
}

fn configured_from_capture(directory: &TempDir, original: &Value) -> Value {
    std::fs::write(directory.path().join("policy.toml"), CONFIG).unwrap();
    let (_output, configured) = replay(
        directory.path(),
        original,
        &["--compiler-advisory-config", "policy.toml"],
    );
    configured
}

pub(super) fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-advisory",
        "--replay-run",
        original["run_directory"].as_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

fn diagnostics(report: &Value) -> &[Value] {
    report["compiler_advisory"]["units"][0]["evaluation"]["diagnostics"]
        .as_array()
        .unwrap()
}
fn decisions(report: &Value) -> &[Value] {
    report["compiler_advisory"]["diagnostic_review"]["decisions"]
        .as_array()
        .unwrap()
}
fn dismiss(directory: &Path, report: &Value, index: usize) -> Value {
    let (output, reviewed) = replay(
        directory,
        report,
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-diagnostic",
            "0",
            &index.to_string(),
            "--review-reason",
            "Independently reviewed fixture finding",
        ],
    );
    assert!(
        output.status.success(),
        "{}\n{}",
        stdout(&output),
        String::from_utf8_lossy(&output.stderr)
    );
    reviewed
}

#[test]
fn all_six_compiler_style_families_use_exact_reviews_without_changing_raw_reports() {
    let (directory, _, raw) = capture(SOURCE, &[]);
    for scenario in [
        movement_and_parameter_renaming_preserve_completed_compiler_reviews_scenario
            as fn(&TempDir, &Value),
        changed_distribution_and_policy_make_compiler_reviews_visible_again_scenario
            as fn(&TempDir, &Value),
        compiler_diagnostic_reviews_keep_check_and_test_contexts_separate_scenario
            as fn(&TempDir, &Value),
        compiler_diagnostic_reviews_preserve_contexts_missing_findings_and_atomic_writes_scenario
            as fn(&TempDir, &Value),
        all_six_compiler_style_families_use_exact_reviews_without_changing_raw_reports_scenario
            as fn(&TempDir, &Value),
    ] {
        reset_review_fixture(&directory);
        let original = configured_from_capture(&directory, &raw);
        scenario(&directory, &original);
    }
}

fn reset_review_fixture(directory: &TempDir) {
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    std::fs::write(directory.path().join("policy.toml"), CONFIG).unwrap();
    for name in ["reviews.json", "baseline.json", "selection.json"] {
        let path = directory.path().join(name);
        if path.exists() {
            std::fs::remove_file(path).unwrap();
        }
    }
}

fn all_six_compiler_style_families_use_exact_reviews_without_changing_raw_reports_scenario(
    directory: &TempDir,
    original: &Value,
) {
    let rules: std::collections::BTreeSet<_> = diagnostics(original)
        .iter()
        .map(|d| d["rule_id"].as_str().unwrap())
        .collect();
    assert_eq!(
        rules,
        [
            "fn_length",
            "fn_params",
            "fn_spacing",
            "mutation_per_target",
            "mutated_targets",
            "side_effect_density"
        ]
        .into_iter()
        .collect()
    );
    let mut reviewed = original.clone();
    for index in 0..diagnostics(original).len() {
        reviewed = dismiss(directory.path(), original, index);
        assert_eq!(decisions(&reviewed)[index]["state"], "hidden_exact");
        assert_eq!(reviewed["matching"], original["matching"]);
        assert_eq!(
            reviewed["compiler_advisory"]["units"][0]["evaluation"],
            original["compiler_advisory"]["units"][0]["evaluation"]
        );
    }
    assert_eq!(reviewed["compiler_advisory"]["findings"], 0);
    assert_eq!(
        reviewed["compiler_advisory"]["units"][0]["visible_diagnostics"],
        serde_json::json!([])
    );
    assert_source_free_reviews(directory.path(), original);
}

fn assert_source_free_reviews(directory: &Path, original: &Value) {
    std::fs::remove_file(directory.join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.join("Cargo.toml")).unwrap();
    let (output, saved) = replay(
        directory,
        original,
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--review-file",
            "reviews.json",
            "--review-status",
            "--fail-on-findings",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    assert!(
        decisions(&saved)
            .iter()
            .all(|row| row["state"] == "hidden_exact")
    );
    assert!(
        saved["review_status"]["records"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["state"] == "hidden_exact")
    );
    let bytes = std::fs::read(directory.join("reviews.json")).unwrap();
    let store: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(store["schema_version"], 9);
    assert_eq!(
        store["compiler_diagnostic_entries"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    assert!(!String::from_utf8(bytes).unwrap().contains("sha256"));
}

fn movement_and_parameter_renaming_preserve_completed_compiler_reviews_scenario(
    directory: &TempDir,
    original: &Value,
) {
    let index = diagnostics(original)
        .iter()
        .position(|d| d["rule_id"] == "fn_params")
        .unwrap();
    dismiss(directory.path(), original, index);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!(
            "\n// movement\n{}",
            SOURCE
                .replace("a:u128", "renamed:u128")
                .replace("x=a", "x=renamed")
        ),
    )
    .unwrap();
    let (_output, current) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-advisory-config",
            "policy.toml",
            "--review-file",
            "reviews.json",
        ],
    );
    let index = diagnostics(&current)
        .iter()
        .position(|d| d["rule_id"] == "fn_params")
        .unwrap();
    assert_eq!(decisions(&current)[index]["state"], "hidden_exact");
}

fn changed_distribution_and_policy_make_compiler_reviews_visible_again_scenario(
    directory: &TempDir,
    original: &Value,
) {
    let index = diagnostics(original)
        .iter()
        .position(|d| d["rule_id"] == "mutation_per_target")
        .unwrap();
    dismiss(directory.path(), original, index);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        SOURCE.replace("x+=2", "y+=2"),
    )
    .unwrap();
    let (_output, current) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-advisory-config",
            "policy.toml",
            "--review-file",
            "reviews.json",
            "--review-status",
        ],
    );
    let changed = diagnostics(&current)
        .iter()
        .position(|d| d["rule_id"] == "mutation_per_target")
        .unwrap();
    assert_eq!(
        diagnostics(&current)[changed]["actual"],
        diagnostics(original)[index]["actual"]
    );
    assert_eq!(decisions(&current)[changed]["state"], "stale");
    assert_eq!(current["review_status"]["records"][0]["state"], "stale");
    assert_changed_policy_review_is_stale(directory, original, index);
}

fn assert_changed_policy_review_is_stale(directory: &TempDir, original: &Value, index: usize) {
    std::fs::write(
        directory.path().join("policy.toml"),
        CONFIG.replace("max_params=1", "max_params=0"),
    )
    .unwrap();
    let (_output, changed_policy) = replay(
        directory.path(),
        original,
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--review-file",
            "reviews.json",
        ],
    );
    assert_eq!(decisions(&changed_policy)[index]["state"], "stale");
}

fn compiler_diagnostic_reviews_keep_check_and_test_contexts_separate_scenario(
    directory: &TempDir,
    original: &Value,
) {
    let index = diagnostics(original)
        .iter()
        .position(|d| d["rule_id"] == "fn_params")
        .unwrap();
    dismiss(directory.path(), original, index);
    let (_output, both) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--all-targets",
            "--compiler-advisory-config",
            "policy.toml",
            "--review-file",
            "reviews.json",
        ],
    );
    assert_eq!(
        decisions(&both)
            .iter()
            .filter(|row| row["state"] == "hidden_exact")
            .count(),
        1
    );
}

fn compiler_diagnostic_reviews_preserve_contexts_missing_findings_and_atomic_writes_scenario(
    directory: &TempDir,
    original: &Value,
) {
    let index = diagnostics(original)
        .iter()
        .position(|d| d["rule_id"] == "fn_params")
        .unwrap();
    dismiss(directory.path(), original, index);
    let path = directory.path().join("reviews.json");
    let bytes = std::fs::read(&path).unwrap();
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-diagnostic",
            "0",
            "999",
            "--review-reason",
            "Invalid selector",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let (output, changed_policy) = replay(
        directory.path(),
        original,
        &["--review-file", "reviews.json", "--review-prune"],
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        changed_policy["review_status"]["records"][0]["state"],
        "stale"
    );
    assert_eq!(changed_policy["review_prune"]["removed_entries"], 0);
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn invalid_suppressions_cannot_be_dismissed_and_lookup_creates_no_store() {
    let source = "// rstyle-allow(unknown): invalid fixture\npub fn original(){}";
    let (directory, _output, original) = capture(source, &[]);
    assert_eq!(diagnostics(&original)[0]["rule_id"], "invalid_suppression");
    let (output, lookup) = replay(
        directory.path(),
        &original,
        &["--review-file", "reviews.json"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(decisions(&lookup)[0]["state"], "unavailable");
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-diagnostic",
            "0",
            "0",
            "--review-reason",
            "Cannot hide invalid suppression",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(!directory.path().join("reviews.json").exists());
}

#[test]
fn hiding_a_compiler_diagnostic_cannot_erase_missing_generated_input() {
    let source = "macro_rules! make {()=>{pub fn generated(){}}} make!(); pub fn original(a:u128,b:u128,c:u128,d:u128,e:u128){}";
    let (directory, _output, original) = capture(source, &[]);
    assert_eq!(diagnostics(&original)[0]["rule_id"], "fn_params");
    let (output, reviewed) = replay(
        directory.path(),
        &original,
        &[
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-diagnostic",
            "0",
            "0",
            "--review-reason",
            "Fixture review with incomplete scope",
            "--fail-on-findings",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(decisions(&reviewed)[0]["state"], "hidden_exact");
    assert_eq!(reviewed["compiler_advisory"]["findings"], 0);
    assert_eq!(reviewed["compiler_advisory"]["complete"], false);
    assert_eq!(reviewed["matching"], original["matching"]);
}

mod baseline;
mod direct_density;
mod negatives;
