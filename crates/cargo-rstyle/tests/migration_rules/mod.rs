use super::*;
mod exclusions;
mod spacing;
mod suppressions;

fn source_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "pub fn many(a: u8, b: u8) -> u8 {\n    let read = || a;\n    read() + b\n}\npub fn empty() {}\npub fn clean(a: u8) -> u8 { a }\npub struct Holder;\nimpl Holder { pub fn arity(&self, a: u8, b: u8) -> u8 { a + b } }\n").unwrap();
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_length]\nmax_lines = 2\n[fn_params]\nmax_params = 1\ncount_self = false\n",
    )
    .unwrap();
    directory
}

fn source_report(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--source-rules",
        "--analysis-config",
        "analysis.toml",
    ];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

fn assessed<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"].as_str().unwrap().ends_with(name))
        .unwrap()
}

#[test]
fn source_rules_find_violations_even_when_the_body_is_clone_excluded() {
    let directory = source_workspace();
    let (output, full) = source_report(directory.path(), &[]);
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        full
    );
    assert_eq!(full["analysis"]["schema"], "rstyle.consumer-analysis/1");
    assert_eq!(full["analysis"]["execution"], "completed");
    assert_eq!(full["analysis"]["scope"]["coverage"], "complete");
    let many = assessed(&full, "many");
    assert_eq!(many["assessments"][0]["actual"], 4);
    assert_eq!(many["assessments"][1]["actual"], 2);
    for rule in many["assessments"].as_array().unwrap() {
        assert_eq!(rule["outcome"], "finding");
        assert_eq!(rule["availability"], "known");
    }
    let graph = full["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["key"] == many["key"])
        .unwrap();
    assert_eq!(graph["outcome"], "excluded");
    assert_eq!(assessed(&full, "empty")["assessments"][1]["actual"], 0);
    assert_eq!(assessed(&full, "arity")["assessments"][1]["actual"], 2);

    let (output, compact) = source_report(directory.path(), &["--report-detail", "compact"]);
    assert!(output.status.success());
    assert_eq!(full["analysis"], compact["analysis"]);
    let (output, report) = source_report(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(report["analysis"]["findings"].as_u64().unwrap() > 0);
}

#[test]
fn source_rule_budget_is_incomplete_without_changing_pair_completion() {
    let directory = source_workspace();
    let (output, report) = source_report(directory.path(), &["--max-analysis-functions", "0"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["analysis"]["execution"], "budget_exhausted");
    assert_eq!(report["analysis"]["scope"]["coverage"], "incomplete");
    assert_eq!(report["analysis"]["scope"]["assessed_functions"], 0);
    assert_eq!(report["analysis"]["findings"], 0);
    assert_eq!(report["pair_evaluation_complete"], true);
    for function in report["analysis"]["functions"].as_array().unwrap() {
        assert_eq!(function["execution"], "budget_exhausted");
        assert_eq!(function["assessments"][0]["outcome"], "not_evaluated");
        assert!(function["assessments"][0]["actual"].is_null());
    }
}

#[test]
fn missing_source_ownership_is_unknown_and_not_requested_is_distinct() {
    let directory = source_workspace();
    let (output, report) = clones(directory.path(), &["--source-rules"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["analysis"]["execution"], "completed");
    assert_eq!(report["analysis"]["scope"]["coverage"], "incomplete");
    assert_eq!(
        assessed(&report, "empty")["assessments"][1]["availability"],
        "unknown"
    );
    let (output, report) = clones(directory.path(), &[]);
    assert!(output.status.success());
    assert_eq!(report["analysis"]["execution"], "not_requested");
    assert!(
        report["analysis"]["functions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn source_rule_policy_counts_receivers_only_when_requested() {
    let directory = source_workspace();
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_params]\nmax_params = 2\ncount_self = true\n",
    )
    .unwrap();
    let (output, report) = source_report(directory.path(), &[]);
    assert!(output.status.success());
    let arity = assessed(&report, "arity");
    assert_eq!(arity["assessments"][1]["actual"], 3);
    assert_eq!(arity["assessments"][1]["outcome"], "finding");
}

#[test]
fn failed_capture_does_not_become_a_clean_empty_source_rule_run() {
    let directory = source_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn broken( {").unwrap();
    let (output, report) = source_report(directory.path(), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["capture_complete"], false);
    assert_eq!(report["analysis"]["execution"], "failed");
    assert_eq!(report["analysis"]["scope"]["coverage"], "incomplete");
    assert!(report["analysis"]["scope"]["expected_functions"].is_null());
}

#[test]
fn source_rule_configuration_rejects_unrelated_policy_before_capture() {
    let directory = source_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn broken( {").unwrap();
    for policy in [
        "[mutation_per_target]\nmax_mutations = 2\n",
        "[fn_params]\nmax_params = -1\n",
    ] {
        std::fs::write(directory.path().join("analysis.toml"), policy).unwrap();
        let output = rstyle(
            directory.path(),
            &[
                "clones",
                "--source-rules",
                "--analysis-config",
                "analysis.toml",
            ],
        );
        assert_eq!(output.status.code(), Some(2));
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("analysis.toml"), "{error}");
        assert!(!error.contains("could not compile"), "{error}");
    }
}
