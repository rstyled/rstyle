//! Typed policy inputs; compiler extraction and source locations stay in CLI tests.
use super::*;
use crate::clones::Handoff;
use crate::clones::analysis::{helper_reports, helpers};
use serde_json::{Value, json};

fn function(body: HirNode, names: &[&str]) -> HirFnFacts {
    let mut result = facts(body);
    result.signature.params.truncate(names.len());
    for (parameter, name) in result.signature.params.iter_mut().zip(names) {
        parameter.symbol = (*name).into();
        parameter.pat.binding = Some((*name).into());
    }
    result
}

fn comparison<'a>(left: &'a HirFnFacts, right: &'a HirFnFacts) -> ComparedPair<'a> {
    let graph = admitted(Some("typed-helper-policy"));
    compare(
        &Endpoint::from_native(&graph, left),
        &Endpoint::from_native(&graph, right),
    )
}

fn row<'a>(
    first: &'a str,
    second: &'a str,
    comparison: &ComparedPair<'a>,
) -> helper_reports::Pair<'a> {
    helper_reports::Pair {
        first,
        second,
        assessment: helpers::evaluate(comparison),
    }
}

fn report(pairs: Vec<helper_reports::Pair<'_>>, permits_success: bool) -> Value {
    let handoff = Handoff::new(None);
    let report = helper_reports::summarize(pairs, &handoff, true, true);
    assert_eq!(report.permits_success(), permits_success);
    serde_json::to_value(report).unwrap()
}

fn assert_complete(report: &Value, findings: usize, pairs: usize) {
    assert_eq!(report["execution"], "completed");
    assert_eq!(report["coverage"], "complete");
    assert_eq!(report["findings"], findings);

    assert_eq!(report["expected_pairs"], pairs);
    assert_eq!(report["assessed_pairs"], pairs);
    assert_eq!(report["evaluated_pairs"], pairs);
    assert_eq!(report["unavailable_pairs"], 0);

    assert!(
        report["unavailable_functions"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    assert_eq!(report["policy"]["minimum_builtin_operations"], 2);
    assert_eq!(report["policy"]["max_literal_holes"], 1);
    assert_eq!(report["policy"]["scalar_type"], "u32");
}

fn assert_negative(case: &str, comparison: &ComparedPair<'_>, operations: usize) {
    assert_eq!(
        comparison.assessment().coverage,
        Coverage::Complete,
        "{case}"
    );
    assert_eq!(
        comparison.helper_inputs().unwrap().builtin_operations,
        [operations; 2],
        "{case}"
    );

    let report = report(vec![row("first", "second", comparison)], true);
    assert_complete(&report, 0, 1);

    let pair = &report["pairs"][0];
    assert_eq!(pair["outcome"], "evaluated_no_finding", "{case}");
    assert_eq!(pair["coverage"], "complete");
    assert_eq!(pair["availability"], "known");
    assert!(pair["proposal"].is_null());
    assert!(!pair["reasons"].as_array().unwrap().is_empty());
}

#[test]
fn identity_forwarding_is_below_the_helper_usefulness_boundary() {
    let left = function(read(10, 1), &["value"]);
    let right = function(read(10, 1), &["input"]);
    assert_negative("identity-forwarding", &comparison(&left, &right), 0);
}

#[test]
fn one_operation_literal_hole_is_below_the_helper_usefulness_boundary() {
    let left = function(binary("+", read(10, 1), literal(11, "1")), &["value"]);
    let right = function(binary("+", read(10, 1), literal(11, "3")), &["input"]);
    assert_negative(
        "one-operation-below-usefulness-boundary",
        &comparison(&left, &right),
        1,
    );
}

#[test]
fn noncommutative_formal_reordering_is_not_a_literal_helper() {
    let left = function(reordered_body(false), &["left", "right"]);
    let right = function(reordered_body(true), &["left", "right"]);

    let compared = comparison(&left, &right);
    assert_eq!(compared.assessment().holes.len(), 2);
    assert!(
        compared
            .assessment()
            .holes
            .iter()
            .all(|hole| hole.kind == HoleKind::FormalRole)
    );

    assert_negative("noncommutative-formal-reordering", &compared, 2);
}

fn reordered_body(swapped: bool) -> HirNode {
    let (first, second) = if swapped { (2, 1) } else { (1, 2) };
    let inner = binary("-", read(10, first), read(11, second));
    let mut outer = binary("*", inner, literal(12, "2"));
    outer.hir.local = 101;

    outer
}

fn masked_body() -> HirNode {
    let inner = binary("^", read(10, 1), read(11, 2));
    let mut outer = binary("&", inner, literal(12, "15"));
    outer.hir.local = 101;

    outer
}

#[test]
fn two_operation_alpha_only_preserves_ordered_formals_and_advisory() {
    let case = "two-operation-alpha-only";
    let left = function(masked_body(), &["value", "mask"]);
    let right = function(masked_body(), &["input", "bits"]);

    let compared = comparison(&left, &right);

    assert_eq!(compared.assessment().coverage, Coverage::Complete, "{case}");
    assert!(compared.assessment().holes.is_empty());

    let report = report(vec![row("first", "second", &compared)], true);
    assert_complete(&report, 1, 1);

    let pair = &report["pairs"][0];
    assert_eq!(pair["outcome"], "finding");
    assert_eq!(pair["coverage"], "complete");
    assert_eq!(pair["availability"], "known");

    let proposal = &pair["proposal"];
    assert_eq!(proposal["builtin_operations"], json!([2, 2]));
    assert_eq!(proposal["literal_arguments"], json!([]));

    let parameters = proposal["formal_parameters"].as_array().unwrap();
    assert_eq!(parameters.len(), 2);
    for (parameter, position) in parameters.iter().zip([0, 1]) {
        assert_eq!(parameter.as_object().unwrap().len(), 2);
        assert_eq!(parameter["position"], position);
        assert_eq!(parameter["scalar_type"], "u32");
    }

    assert!(proposal["correspondence_count"].as_u64().unwrap() > 0);

    let advisory = proposal["advisory"].as_str().unwrap();
    assert!(!advisory.is_empty());
    assert!(advisory.contains("overflow"));
    assert!(advisory.contains("before editing") || advisory.contains("manual"));
    assert!(advisory.contains("not an automatic safe rewrite"));
}

#[test]
fn pairwise_evidence_not_transitive_keeps_three_distinct_comparisons() {
    let case = "pairwise-evidence-not-transitive";
    let a = function(two_operations("1"), &["value"]);
    let b = function(two_operations("3"), &["value"]);
    let mut c = function(two_operations("3"), &["value"]);
    c.body.children[1].lit = Some("4".into());
    let ab = comparison(&a, &b);
    let bc = comparison(&b, &c);
    let ac = comparison(&a, &c);
    for (compared, holes) in [(&ab, 1), (&bc, 1), (&ac, 2)] {
        assert_eq!(compared.assessment().coverage, Coverage::Complete, "{case}");
        assert_eq!(compared.assessment().holes.len(), holes);
        assert!(
            compared
                .assessment()
                .holes
                .iter()
                .all(|hole| hole.kind == HoleKind::Literal)
        );
    }

    let report = report(
        vec![row("a", "b", &ab), row("b", "c", &bc), row("a", "c", &ac)],
        true,
    );
    assert_complete(&report, 2, 3);
    for (pair, outcome) in report["pairs"].as_array().unwrap().iter().zip([
        "finding",
        "finding",
        "evaluated_no_finding",
    ]) {
        assert_eq!(pair["outcome"], outcome);
        if outcome == "evaluated_no_finding" {
            assert!(pair["proposal"].is_null());
        }
    }
    assert!(report.get("groups").is_none());
}

fn wide_node(node: HirNode) -> HirNode {
    HirNode {
        ty: Some(wide_type()),
        dispatch: node.dispatch.map(|dispatch| Dispatch {
            operand_tys: vec!["u64".into(); dispatch.operand_tys.len()],
            result_ty: "u64".into(),
            ..dispatch
        }),
        children: node.children.into_iter().map(wide_node).collect(),
        ..node
    }
}

fn wide_type() -> ResolvedTy {
    ResolvedTy {
        normalized: Some("u64".into()),
        observed: "u64".into(),
        spelling: "u64".into(),
    }
}

fn wide_function(literal: &str, name: &str) -> HirFnFacts {
    let facts = function(two_operations(literal), &[name]);
    let params = facts
        .signature
        .params
        .into_iter()
        .map(|parameter| rstyle_pdg::frontend::ParamFact {
            pat: wide_node(parameter.pat),
            ty: wide_type(),
            ..parameter
        })
        .collect();

    HirFnFacts {
        body: wide_node(facts.body),
        signature: rstyle_pdg::frontend::SignatureFacts {
            params,
            return_ty: wide_type(),
            ..facts.signature
        },
        ..facts
    }
}

#[test]
fn complete_u64_native_comparison_is_unavailable_to_u32_helper_policy() {
    let left = wide_function("1", "value");
    let right = wide_function("3", "input");

    let compared = comparison(&left, &right);

    assert_eq!(compared.assessment().coverage, Coverage::Complete);
    assert_eq!(compared.assessment().holes.len(), 1);
    assert_eq!(compared.assessment().holes[0].kind, HoleKind::Literal);

    let report = report(vec![row("first", "second", &compared)], false);
    assert_eq!(report["coverage"], "incomplete");
    assert_eq!(report["findings"], 0);

    assert_eq!(report["assessed_pairs"], 1);
    assert_eq!(report["evaluated_pairs"], 0);
    assert_eq!(report["unavailable_pairs"], 1);

    let pair = &report["pairs"][0];
    assert_eq!(pair["outcome"], "not_evaluated");
    assert_eq!(pair["availability"], "unsupported");
    assert_eq!(pair["coverage"], "incomplete");
    assert!(pair["proposal"].is_null());
    assert!(!pair["reasons"].as_array().unwrap().is_empty());
}
