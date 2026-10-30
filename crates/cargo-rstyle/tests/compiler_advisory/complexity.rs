use super::*;

const SOURCE: &str = r#"
pub fn straight() -> u128 { 1 }
pub fn nested(a: bool, b: bool) { if a { if b {} } }
pub fn flat(a: bool, b: bool) { if a {} if b {} }
pub fn recursion(x: u128) -> u128 { if x == 0 { return 0; } recursion(x - 1) }
"#;

fn evaluation(report: &Value) -> &Value {
    &report["compiler_advisory"]["units"][0]["evaluation"]
}

fn captured<'a>(report: &'a Value, name: &str) -> &'a Value {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == name)
        .unwrap();
    &function["facts"]["build"]["frontend_options"][rstyle_core::advisory::KEY]["facts"]
}

#[test]
fn advisory_complexity_reuses_original_constructs_and_core_scores_without_strict_admission() {
    let (_directory, output, report) = capture(SOURCE, &[]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(
        report["compiler_advisory"]["metrics"],
        "rstyle.compiler-style-metrics/7"
    );
    assert_eq!(evaluation(&report)["summary"]["complexity_measured"], 4);
    assert_eq!(evaluation(&report)["summary"]["complexity_unavailable"], 0);
    for (name, cyclomatic, cognitive) in [
        ("straight", 1, 0),
        ("nested", 3, 3),
        ("flat", 3, 2),
        ("recursion", 2, 2),
    ] {
        let score = &captured(&report, name)["complexity"];
        assert_eq!(score["policy_version"], 1);
        assert_eq!(score["cyclomatic"], cyclomatic, "{name}");
        assert_eq!(score["cognitive"], cognitive, "{name}");
    }
    let nested = &captured(&report, "nested")["complexity"]["contributions"];
    assert_eq!(nested[0]["nesting"], 0);
    assert_eq!(nested[1]["nesting"], 1);
    assert_eq!(nested[1]["cognitive"], 2);
    let recursion = &captured(&report, "recursion")["complexity"]["contributions"];
    assert!(
        recursion
            .as_array()
            .unwrap()
            .iter()
            .any(|point| point["kind"] == "recursion" && point["cognitive"] == 1)
    );
    assert!(
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|function| {
                function["facts"]["name"] == "recursion" && function["outcome"] != "admitted"
            })
    );
}

#[test]
fn replayed_complexity_limits_use_completed_measurements_without_original_sources() {
    let (directory, output, original) = capture(SOURCE, &[]);
    assert!(output.status.success(), "{}", stdout(&output));
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    for path in ["Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    std::fs::write(
        directory.path().join("policy.toml"),
        "[complexity]\nmax_cyclomatic=2\nmax_cognitive=2\n",
    )
    .unwrap();
    let (output, replayed) = super::reviews::replay(
        directory.path(),
        &original,
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--fail-on-findings",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    let diagnostics = evaluation(&replayed)["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 3);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|row| row["rule_id"] == "cyclomatic_complexity")
            .count(),
        2
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|row| row["rule_id"] == "cognitive_complexity")
            .count(),
        1
    );
    assert!(
        diagnostics
            .iter()
            .all(|row| row["actual"] == 3 && row["limit"] == 2)
    );
    assert_eq!(original["matching"], replayed["matching"]);
    assert_eq!(original["accepted_pairs"], replayed["accepted_pairs"]);
    for name in ["straight", "nested", "flat", "recursion"] {
        assert_eq!(
            captured(&original, name)["complexity"],
            captured(&replayed, name)["complexity"]
        );
    }
}

#[test]
fn complexity_diagnostics_honor_existing_suppressions_and_source_exclusions() {
    let source = "pub fn nested(a:bool,b:bool) { if a { if b {} } }\n// rstyle-allow(cyclomatic_complexity, cognitive_complexity): reviewed fixture\npub fn allowed(a:bool,b:bool) { if a { if b {} } }";
    let (directory, output, original) = capture(source, &[]);
    assert!(output.status.success(), "{}", stdout(&output));
    std::fs::write(
        directory.path().join("policy.toml"),
        "[complexity]\nmax_cyclomatic=2\nmax_cognitive=2\n",
    )
    .unwrap();
    let policy = ["--compiler-advisory-config", "policy.toml"];
    let (output, report) = super::reviews::replay(directory.path(), &original, &policy);
    assert!(output.status.success(), "{}", stdout(&output));
    let diagnostics = evaluation(&report)["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(|row| row["line"] == 1));
    assert_eq!(
        evaluation(&report)["fix_groups"].as_array().unwrap().len(),
        1
    );
    std::fs::write(
        directory.path().join("policy.toml"),
        "exclude=['src/**']\n[complexity]\nmax_cyclomatic=2\nmax_cognitive=2\n",
    )
    .unwrap();
    let (output, excluded) = super::reviews::replay(directory.path(), &original, &policy);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(evaluation(&excluded)["diagnostics"], serde_json::json!([]));
    assert_eq!(evaluation(&excluded)["summary"]["functions"], 0);
}

fn review_policy(directory: &Path, limit: usize) {
    std::fs::write(
        directory.join("policy.toml"),
        format!("[complexity]\nmax_cyclomatic={limit}\nmax_cognitive={limit}\n"),
    )
    .unwrap();
}

fn dismiss_complexity(directory: &Path, original: &Value) -> Value {
    let (output, report) = super::reviews::replay(
        directory,
        original,
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-diagnostic",
            "0",
            "0",
            "--review-reason",
            "Independently reviewed nested complexity",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    report
}

#[test]
fn completed_complexity_reviews_hide_exact_findings_and_reopen_changed_limits() {
    let source = "pub fn nested(a:bool,b:bool) { if a { if b {} } }";
    let (directory, output, original) = capture(source, &[]);
    assert!(output.status.success(), "{}", stdout(&output));
    review_policy(directory.path(), 2);
    let policy = ["--compiler-advisory-config", "policy.toml"];
    let (output, configured) = super::reviews::replay(directory.path(), &original, &policy);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(
        evaluation(&configured)["diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let reviewed = dismiss_complexity(directory.path(), &original);
    let raw = &evaluation(&configured)["diagnostics"];
    assert_eq!(evaluation(&reviewed)["diagnostics"], *raw);
    let review = &reviewed["compiler_advisory"]["diagnostic_review"];
    assert_eq!(review["decisions"][0]["state"], "hidden_exact");
    let mut current = policy.to_vec();
    current.extend(["--review-file", "reviews.json"]);
    let (output, unchanged) = super::reviews::replay(directory.path(), &original, &current);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(
        unchanged["compiler_advisory"]["diagnostic_review"]["decisions"][0]["state"],
        "hidden_exact"
    );
    assert_eq!(evaluation(&unchanged)["diagnostics"], *raw);
    review_policy(directory.path(), 1);
    let (output, changed) = super::reviews::replay(directory.path(), &original, &current);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(
        changed["compiler_advisory"]["diagnostic_review"]["decisions"][0]["state"],
        "stale"
    );
    assert_eq!(evaluation(&changed)["diagnostics"][0]["actual"], 3);
    assert_eq!(evaluation(&changed)["diagnostics"][0]["limit"], 1);
    assert_eq!(changed["matching"], original["matching"]);
}
