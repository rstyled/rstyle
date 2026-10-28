use super::*;

fn frozen_case(id: &str) -> Value {
    let labels: Value = serde_json::from_str(include_str!(
        "../../../../eval/clones-capability/proposal-review-cases.json"
    ))
    .unwrap();
    labels["proposal_cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap()
        .clone()
}

fn workspace(id: &str) -> TempDir {
    let directory = clone_workspace();
    let source = frozen_case(id)["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|source| source.as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    directory
}

fn requested(directory: &Path) -> (Output, Value) {
    clones(
        directory,
        &["--extraction-profile", "hir-data-v3", "--helper-proposals"],
    )
}

fn named_key<'a>(report: &'a Value, suffix: &str) -> &'a str {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| {
            function["name"]
                .as_str()
                .or_else(|| function["facts"]["name"].as_str())
                .unwrap()
                .ends_with(suffix)
        })
        .unwrap()["key"]
        .as_str()
        .unwrap()
}

fn pair<'a>(report: &'a Value, left: &str, right: &str) -> &'a Value {
    let (left, right) = (named_key(report, left), named_key(report, right));
    report["helper_proposals"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| {
            (pair["first"] == left && pair["second"] == right)
                || (pair["first"] == right && pair["second"] == left)
        })
        .unwrap()
}

fn assert_complete(report: &Value, findings: usize, pairs: usize) {
    let helper = &report["helper_proposals"];
    assert_eq!(helper["execution"], "completed");
    assert_eq!(helper["coverage"], "complete");
    assert_eq!(helper["findings"], findings);
    assert_eq!(helper["expected_pairs"], pairs);
    assert_eq!(helper["assessed_pairs"], pairs);
    assert_eq!(helper["evaluated_pairs"], pairs);
    assert_eq!(helper["unavailable_pairs"], 0);
    assert!(
        helper["unavailable_functions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(helper["policy"]["minimum_builtin_operations"], 2);
    assert_eq!(helper["policy"]["max_literal_holes"], 1);
    assert_eq!(helper["policy"]["scalar_type"], "u32");
}

#[test]
fn frozen_two_operation_helpers_retain_native_literal_evidence() {
    for (id, holes) in [
        ("two-operation-literal-hole", 1),
        ("two-operation-alpha-only", 0),
    ] {
        let directory = workspace(id);
        let (output, report) = requested(directory.path());
        assert!(output.status.success(), "{}", stdout(&output));
        assert_complete(&report, 1, 1);
        let row = pair(&report, "first", "second");
        assert_eq!(row["outcome"], "finding");
        assert_eq!(row["coverage"], "complete");
        assert_eq!(row["availability"], "known");
        let proposal = &row["proposal"];
        assert_eq!(proposal["builtin_operations"], serde_json::json!([2, 2]));
        assert!(proposal["correspondence_count"].as_u64().unwrap() > 0);
        let advisory = proposal["advisory"].as_str().unwrap();
        assert!(!advisory.is_empty());
        assert!(advisory.contains("overflow"));
        assert!(advisory.contains("before editing") || advisory.contains("manual"));
        assert!(advisory.contains("not an automatic safe rewrite"));
        let arguments = proposal["literal_arguments"].as_array().unwrap();
        assert_eq!(arguments.len(), holes);
        if holes == 1 {
            assert_literal_arguments(&arguments[0]);
        }
        assert_eq!(report["pair_explanations"]["coverage"], "complete");
    }
}

fn assert_literal_arguments(argument: &Value) {
    assert!(!argument["position"].is_null());
    let mut values = Vec::new();
    for side in ["left", "right"] {
        let endpoint = &argument[side];
        assert!(!endpoint["hir"].is_null());
        assert!(!endpoint["span"].is_null());
        assert_eq!(endpoint["scalar_type"], "u32");
        values.push(
            endpoint["value"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| endpoint["value"].to_string()),
        );
    }
    values.sort();
    assert_eq!(values, ["1", "3"]);
}

#[test]
fn frozen_forwarding_small_and_reordered_formal_negatives_stay_clean() {
    for id in [
        "identity-forwarding",
        "one-operation-below-usefulness-boundary",
        "noncommutative-formal-reordering",
    ] {
        let directory = workspace(id);
        let (output, report) = requested(directory.path());
        assert!(output.status.success(), "{id}: {}", stdout(&output));
        assert_complete(&report, 0, 1);
        let row = pair(&report, "first", "second");
        assert_eq!(row["outcome"], "evaluated_no_finding");
        assert!(row["proposal"].is_null());
    }
}

#[test]
fn unknown_calls_and_mutation_remain_visible_without_admitted_pairs() {
    for id in ["unknown-call-boundary", "observable-effects-boundary"] {
        let directory = workspace(id);
        let (output, report) = requested(directory.path());
        assert_eq!(output.status.code(), Some(2), "{id}: {}", stdout(&output));
        let helper = &report["helper_proposals"];
        assert_eq!(helper["coverage"], "incomplete");
        assert_eq!(helper["findings"], 0);
        let unavailable = helper["unavailable_functions"].as_array().unwrap();
        for suffix in ["first", "second"] {
            let function = unavailable
                .iter()
                .find(|function| function["name"].as_str().unwrap().ends_with(suffix))
                .unwrap();
            assert!(!function["key"].is_null());
            assert_ne!(function["outcome"], "available");
            assert!(!function["reasons"].as_array().unwrap().is_empty());
        }
    }
}

#[test]
fn connected_pairs_do_not_establish_a_three_member_helper() {
    let directory = workspace("pairwise-evidence-not-transitive");
    let (output, report) = requested(directory.path());
    assert!(output.status.success(), "{}", stdout(&output));
    assert_complete(&report, 2, 3);
    for (left, right, outcome) in [
        ("a", "b", "finding"),
        ("b", "c", "finding"),
        ("a", "c", "evaluated_no_finding"),
    ] {
        let row = pair(&report, left, right);
        assert_eq!(row["outcome"], outcome);
        if outcome == "evaluated_no_finding" {
            assert!(row["proposal"].is_null());
        }
    }
    assert!(report["helper_proposals"].get("groups").is_none());
}

#[test]
fn helper_opt_in_is_consistent_in_full_compact_default_and_failure_modes() {
    let directory = workspace("two-operation-literal-hole");
    let (output, full) = requested(directory.path());
    assert!(output.status.success());
    let (output, compact) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--helper-proposals",
            "--report-detail",
            "compact",
        ],
    );
    assert!(output.status.success());
    assert_eq!(full["helper_proposals"], compact["helper_proposals"]);
    assert_eq!(full["pair_explanations"], compact["pair_explanations"]);
    let (output, default) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    assert_eq!(default["helper_proposals"]["execution"], "not_requested");
    assert_eq!(default["matching"], full["matching"]);
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--helper-proposals",
            "--fail-on-findings",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_complete(&report, 1, 1);
}

#[test]
fn matching_budget_is_not_an_empty_complete_helper_assessment() {
    let directory = workspace("two-operation-literal-hole");
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--helper-proposals",
            "--max-pairs",
            "0",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    let helper = &report["helper_proposals"];
    assert_eq!(helper["coverage"], "incomplete");
    assert!(helper["expected_pairs"].is_null());
    assert_eq!(helper["assessed_pairs"], 0);
    assert_eq!(helper["findings"], 0);
}

#[test]
fn helper_text_presents_advisory_source_located_findings() {
    let directory = workspace("two-operation-literal-hole");
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--helper-proposals",
        ],
    );
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.to_lowercase().contains("helper"), "{text}");
    assert!(text.to_lowercase().contains("advisory"), "{text}");
    assert!(text.contains("src/lib.rs"), "{text}");
}

fn source_workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    directory
}

#[test]
fn complete_u64_native_comparison_is_unavailable_to_u32_helper_policy() {
    let directory = source_workspace(
        "pub fn first(value:u64)->u64 { (value+1)*2 }\npub fn second(input:u64)->u64 { (input+3)*2 }\n",
    );
    let (output, report) = requested(directory.path());
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_eq!(report["pair_evaluation_complete"], true);
    assert_eq!(report["pair_explanations"]["coverage"], "complete");
    assert_eq!(report["pair_explanations"]["complete_pairs"], 1);
    let helper = &report["helper_proposals"];
    assert_eq!(helper["coverage"], "incomplete");
    assert_eq!(helper["findings"], 0);
    assert_eq!(helper["evaluated_pairs"], 0);
    assert_eq!(helper["unavailable_pairs"], 1);
    let row = pair(&report, "first", "second");
    assert_eq!(row["outcome"], "not_evaluated");
    assert_eq!(row["availability"], "unsupported");
    assert!(!row["reasons"].as_array().unwrap().is_empty());
}

#[test]
fn native_operator_and_shape_differences_do_not_become_literal_helpers() {
    for alternate in ["(input-1)*2", "input+(1*2)"] {
        let source = format!(
            "pub fn first(value:u32)->u32 {{ (value+1)*2 }}\npub fn second(input:u32)->u32 {{ {alternate} }}\n"
        );
        let directory = source_workspace(&source);
        let (output, report) = requested(directory.path());
        assert!(output.status.success(), "{}", stdout(&output));
        assert_eq!(report["pair_evaluation_complete"], true);
        assert_eq!(report["helper_proposals"]["coverage"], "complete");
        assert_eq!(report["helper_proposals"]["findings"], 0);
        let assessed = report["helper_proposals"]["pairs"].as_array().unwrap();
        if assessed.is_empty() {
            assert!(report["accepted_pairs"].as_array().unwrap().is_empty());
        } else {
            assert_eq!(assessed.len(), 1);
            let row = pair(&report, "first", "second");
            assert_eq!(row["outcome"], "evaluated_no_finding");
            assert_eq!(row["availability"], "known");
            assert!(row["proposal"].is_null());
            assert!(!row["reasons"].as_array().unwrap().is_empty());
            assert_eq!(report["pair_explanations"]["complete_pairs"], 1);
        }
    }
}

#[test]
fn supported_helper_finding_survives_an_unavailable_owner_in_the_same_capture() {
    let directory = source_workspace(
        "pub fn first(value:u32)->u32 { (value+1)*2 }\npub fn second(input:u32)->u32 { (input+3)*2 }\npub fn opaque(value:u32,callback:fn(u32)->u32)->u32 { (callback(value)+1)*2 }\n",
    );
    let (output, report) = requested(directory.path());
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_eq!(report["pair_evaluation_complete"], true);
    let helper = &report["helper_proposals"];
    assert_eq!(helper["coverage"], "incomplete");
    assert_eq!(helper["findings"], 1);
    assert_eq!(pair(&report, "first", "second")["outcome"], "finding");
    let unavailable = helper["unavailable_functions"].as_array().unwrap();
    let opaque = unavailable
        .iter()
        .find(|function| function["name"].as_str().unwrap().ends_with("opaque"))
        .unwrap();
    assert!(!opaque["reasons"].as_array().unwrap().is_empty());
    assert!(!helper["reasons"].as_array().unwrap().is_empty());
}
