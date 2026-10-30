use super::*;

fn guard_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        include_str!("fixtures/lib.rs"),
    )
    .unwrap();
    std::fs::write(
        directory.path().join("src/excluded.rs"),
        include_str!("fixtures/excluded.rs"),
    )
    .unwrap();

    std::fs::write(
        directory.path().join("src/spacing.rs"),
        include_str!("fixtures/spacing.rs"),
    )
    .unwrap();

    directory
}

fn guard<'a>(report: &'a Value, suffix: &str) -> &'a Value {
    &report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"].as_str().unwrap().ends_with(suffix))
        .unwrap()["guard_clause"]
}

#[test]
fn frozen_guards_are_advisory_and_opaque_source_keeps_scope_incomplete() {
    let directory = guard_workspace();
    let args = ["--extraction-profile", "hir-data-v3", "--guard-clauses"];
    let (output, full) = clones(directory.path(), &args);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(guard(&full, "nested_positive")["outcome"], "finding");
    assert_eq!(
        guard(&full, "nested_positive")["opportunity"]["fallback_branch"],
        "else"
    );
    for name in ["already_flat", "both_branches_do_work"] {
        assert_eq!(guard(&full, name)["outcome"], "evaluated_no_finding");
    }
    assert_eq!(
        guard(&full, "unsupported_body_parameters")["outcome"],
        "not_evaluated"
    );
    assert_eq!(
        guard(&full, "unsupported_body_parameters")["availability"],
        "unsupported"
    );
    assert_eq!(full["analysis"]["findings"], 1);
    assert_eq!(full["analysis"]["scope"]["coverage"], "incomplete");
    assert!(full["analysis"]["effective_config"].is_null());
    for function in full["analysis"]["functions"].as_array().unwrap() {
        assert!(function["assessments"].as_array().unwrap().is_empty());
    }
    let (output, compact) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--guard-clauses",
            "--report-detail",
            "compact",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(full["analysis"], compact["analysis"]);
}

#[test]
fn complete_guard_findings_support_opt_in_failure_and_source_text() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "pub fn nested(value: u32) -> u32 { if value > 0 { let next = value + 1; next + 1 } else { 0 } }").unwrap();
    let args = ["--extraction-profile", "hir-data-v3", "--guard-clauses"];
    let (output, report) = clones(directory.path(), &args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(report["analysis"]["scope"]["coverage"], "complete");
    assert_eq!(report["analysis"]["findings"], 1);
    let (output, _) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--guard-clauses",
            "--fail-on-findings",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--guard-clauses",
        ],
    );
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("guard_clause")
            && text.contains("src/lib.rs")
            && text.contains("review lifetimes, drops"),
        "{text}"
    );
}

#[test]
fn guard_budget_and_missing_source_do_not_become_clean_results() {
    let directory = guard_workspace();
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--guard-clauses",
            "--max-analysis-functions",
            "0",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["analysis"]["execution"], "budget_exhausted");
    assert_eq!(
        guard(&report, "nested_positive")["execution"],
        "budget_exhausted"
    );
    assert_eq!(
        guard(&report, "nested_positive")["outcome"],
        "not_evaluated"
    );
    let (output, report) = clones(directory.path(), &["--guard-clauses"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(guard(&report, "nested_positive")["availability"], "unknown");
    assert_eq!(report["analysis"]["scope"]["coverage"], "incomplete");
}

#[test]
fn finding_controls_require_a_requested_local_analysis() {
    let directory = clone_workspace();
    for flag in ["--fail-on-findings", "--max-analysis-functions"] {
        let mut args = vec!["clones", flag];
        if flag == "--max-analysis-functions" {
            args.push("0");
        }
        let output = rstyle(directory.path(), &args);
        assert_eq!(output.status.code(), Some(2));
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("--source-rules") && error.contains("--guard-clauses"),
            "{error}"
        );
    }
}

#[test]
fn guard_advice_does_not_require_the_called_body_to_be_clone_admitted() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "fn opaque(value: u32) -> u32 { std::hint::black_box(value) }\npub fn nested(value: u32) -> u32 { if value > 0 { let next = opaque(value); next + 1 } else { 0 } }").unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--guard-clauses"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(guard(&report, "nested")["outcome"], "finding");
    let key = &report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"].as_str().unwrap().ends_with("nested"))
        .unwrap()["key"];
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| &function["key"] == key)
        .unwrap();
    assert_eq!(function["outcome"], "excluded");
    assert_eq!(report["analysis"]["scope"]["coverage"], "complete");
}

mod resources;

mod closures;

mod reviews;
