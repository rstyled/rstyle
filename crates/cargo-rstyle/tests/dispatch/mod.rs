use super::*;

fn case(id: &str) -> Value {
    let labels: Value = serde_json::from_str(include_str!(
        "../../../../eval/clones-capability/enum-dispatch-cases.json"
    ))
    .unwrap();
    labels["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap()
        .clone()
}

fn workspace(id: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        case(id)["source"].as_str().unwrap(),
    )
    .unwrap();
    directory
}

fn requested(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec!["--extraction-profile", "hir-data-v3", "--enum-dispatch"];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

fn function<'a>(report: &'a Value, suffix: &str) -> &'a Value {
    report["enum_dispatch"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"].as_str().unwrap().ends_with(suffix))
        .unwrap()
}

fn assert_complete(report: &Value, findings: usize) {
    let consumer = &report["enum_dispatch"];
    assert_eq!(consumer["execution"], "completed");
    assert_eq!(consumer["coverage"], "complete");
    assert_eq!(consumer["findings"], findings);
    assert_eq!(
        consumer["assessed_functions"],
        consumer["expected_functions"]
    );
    assert_eq!(
        consumer["assessed_functions"].as_u64().unwrap() as usize,
        consumer["functions"].as_array().unwrap().len()
    );
}

#[test]
fn signature_dispatch_opportunities_do_not_require_supported_method_bodies() {
    for id in [
        "local-inherent-methods-opaque-body",
        "unrelated-trait-does-not-suppress",
    ] {
        let directory = workspace(id);
        let (output, report) = requested(directory.path(), &[]);
        assert!(output.status.success(), "{id}: {}", stdout(&output));
        assert_complete(&report, 1);
        let dispatch = function(&report, "dispatch");
        assert_eq!(dispatch["outcome"], "finding");
        assert_eq!(dispatch["coverage"], "complete");
        assert_proposal_source(&dispatch["proposal"], case(id)["source"].as_str().unwrap());
        let opaque = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| {
                function["facts"]["name"]
                    .as_str()
                    .unwrap()
                    .ends_with("amount")
            })
            .unwrap();
        assert_eq!(opaque["outcome"], "excluded");
        assert_eq!(
            function(&report, "amount")["outcome"],
            "evaluated_no_finding"
        );
    }
}

#[test]
fn frozen_trait_and_receiver_counterexamples_are_evaluated_without_proposals() {
    for id in [
        "shared-existing-trait",
        "different-ordinary-arity",
        "wrong-arm-receiver",
        "same-free-callee",
        "no-dispatch-usage",
    ] {
        let directory = workspace(id);
        let (output, report) = requested(directory.path(), &[]);
        assert!(output.status.success(), "{id}: {}", stdout(&output));
        assert_complete(&report, 0);
        let labels = case(id);
        let assessed = function(&report, labels["function"].as_str().unwrap());
        assert_eq!(assessed["outcome"], "evaluated_no_finding");
        assert_eq!(assessed["coverage"], "complete");
        assert!(assessed["proposal"].is_null());
        if id == "shared-existing-trait" {
            assert!(!assessed["reason"].as_str().unwrap().is_empty());
        }
    }
}

#[test]
fn opaque_dispatch_arm_structure_is_explicitly_incomplete() {
    for id in ["guarded-duplicate-variant", "macro-in-dispatch-arm"] {
        let directory = workspace(id);
        let (output, report) = requested(directory.path(), &[]);
        assert_eq!(output.status.code(), Some(2), "{id}: {}", stdout(&output));
        assert_eq!(report["enum_dispatch"]["coverage"], "incomplete");
        assert_eq!(report["enum_dispatch"]["findings"], 0);
        let assessed = function(&report, "dispatch");
        assert_eq!(assessed["outcome"], "not_evaluated");
        assert_eq!(assessed["coverage"], "incomplete");
        assert!(!assessed["reason"].as_str().unwrap().is_empty());
        assert!(assessed["proposal"].is_null());
    }
}

#[test]
fn dispatch_reports_preserve_full_compact_default_and_opt_in_failure_semantics() {
    let directory = workspace("local-inherent-methods-opaque-body");
    let (output, full) = requested(directory.path(), &[]);
    assert!(output.status.success());
    let (output, compact) = requested(directory.path(), &["--report-detail", "compact"]);
    assert!(output.status.success());
    assert_eq!(full["enum_dispatch"], compact["enum_dispatch"]);
    assert_eq!(full["matching"], compact["matching"]);
    let (output, default) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    assert_eq!(default["enum_dispatch"]["execution"], "not_requested");
    assert_eq!(default["matching"], full["matching"]);
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    assert_complete(&report, 1);
}

#[test]
fn r0_missing_owned_source_does_not_become_a_clean_dispatch_assessment() {
    let directory = workspace("local-inherent-methods-opaque-body");
    let (output, report) = clones(directory.path(), &["--enum-dispatch"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_eq!(report["enum_dispatch"]["coverage"], "incomplete");
    assert_eq!(report["enum_dispatch"]["findings"], 0);
    let assessed = function(&report, "dispatch");
    assert_eq!(assessed["outcome"], "not_evaluated");
    assert!(!assessed["reason"].as_str().unwrap().is_empty());
}

#[test]
fn dispatch_replay_uses_original_owned_sources_without_external_tools() {
    let directory = workspace("local-inherent-methods-opaque-body");
    let (output, original) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let executable = cargo_bin("cargo-rstyle").canonicalize().unwrap();
    let output = diagnostic_process::output(
        Command::new(executable)
            .current_dir(directory.path())
            .args([
                "clones",
                "--format",
                "json",
                "--extraction-profile",
                "hir-data-v3",
                "--enum-dispatch",
                "--replay-run",
                original["run_directory"].as_str().unwrap(),
                "--fail-on-findings",
            ])
            .env("PATH", directory.path().join("no-tools"))
            .env("CARGO", directory.path().join("no-cargo"))
            .env("RSTYLE_DRIVER", directory.path().join("no-driver")),
    )
    .unwrap();
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["verification_policy"], "strict-replay");
    assert_complete(&report, 1);
    assert_eq!(report["matching"], original["matching"]);
    assert_eq!(report["functions"], original["functions"]);
}

#[test]
fn dispatch_budget_retains_unassessed_owners_without_claiming_empty_scope() {
    let directory = workspace("local-inherent-methods-opaque-body");
    let (output, report) = requested(directory.path(), &["--max-analysis-functions", "0"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    let consumer = &report["enum_dispatch"];
    assert_eq!(consumer["execution"], "budget_exhausted");
    assert_eq!(consumer["coverage"], "incomplete");
    assert_eq!(consumer["assessed_functions"], 0);
    assert!(consumer["expected_functions"].as_u64().unwrap() > 0);
    assert_eq!(consumer["findings"], 0);
    for assessed in consumer["functions"].as_array().unwrap() {
        assert_eq!(assessed["execution"], "budget_exhausted");
        assert_eq!(assessed["outcome"], "not_evaluated");
        assert!(assessed["proposal"].is_null());
    }
}

#[test]
fn dispatch_findings_survive_an_unsupported_neighbor_without_clean_exit() {
    let directory = workspace("local-inherent-methods-opaque-body");
    let mut source = case("local-inherent-methods-opaque-body")["source"]
        .as_str()
        .unwrap()
        .to_owned();
    source.push_str(" pub fn guarded(item:&Item)->u32 { match item { Item::A(a) if a.value>0=>a.measure(), Item::A(a)=>a.measure(), Item::B(b)=>b.amount() } }");
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_eq!(report["enum_dispatch"]["findings"], 1);
    assert_eq!(report["enum_dispatch"]["coverage"], "incomplete");
    assert_eq!(function(&report, "dispatch")["outcome"], "finding");
    assert_eq!(function(&report, "guarded")["outcome"], "not_evaluated");
}

fn original_span<'a>(source: &'a str, span: &Value) -> &'a str {
    &source[span["start"].as_u64().unwrap() as usize..span["end"].as_u64().unwrap() as usize]
}

fn assert_proposal_source(proposal: &Value, source: &str) {
    assert_eq!(proposal["enum_name"], "Item");
    assert_eq!(proposal["signature"]["shared_self"], true);
    assert_eq!(proposal["signature"]["ordinary_parameters"], 0);
    assert_eq!(proposal["signature"]["return_type"], "u32");
    assert!(original_span(source, &proposal["caller_header"]).contains("fn dispatch"));
    assert!(original_span(source, &proposal["enum_header"]).contains("enum Item"));
    let arms = proposal["arms"].as_array().unwrap();
    assert_eq!(arms.len(), 2);
    for (arm, (variant, owner, method)) in arms
        .iter()
        .zip([("A", "Alpha", "measure"), ("B", "Beta", "amount")])
    {
        assert_eq!(arm["variant"], variant);
        assert_eq!(arm["receiver_type"], owner);
        assert_eq!(arm["method"], method);
        assert_eq!(arm["origin"]["kind"], "inherent");
        assert!(original_span(source, &arm["callee_header"]).contains(&format!("fn {method}")));
        assert!(original_span(source, &arm["arm_span"]).contains(&format!("Item::{variant}")));
    }
}

#[test]
fn receiver_competition_requires_resolved_origin_instead_of_false_trait_suppression() {
    let directory = workspace("shared-existing-trait");
    let mut source = case("shared-existing-trait")["source"]
        .as_str()
        .unwrap()
        .to_owned();
    source.push_str(" impl Alpha { pub fn measure(&mut self)->u32 {3} }");
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = requested(directory.path(), &[]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    let dispatcher = function(&report, "dispatch");
    assert_eq!(dispatcher["outcome"], "not_evaluated");
    assert_eq!(dispatcher["availability"], "unsupported");
    assert!(dispatcher["reason"].as_str().unwrap().contains("Competing"));
}
