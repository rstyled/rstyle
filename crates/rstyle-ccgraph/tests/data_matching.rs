mod common;

use rstyle_ccgraph::{
    CcgraphConfig, ImportedFunction, InterfaceElement, MatchingProfile, evaluate_all,
};
use rstyle_pdg::{EdgeFamily, NodeCategory};

fn function(key: &str, data: bool) -> ImportedFunction {
    let mut function = common::stage_function(
        key,
        vec![
            common::plain_node("a", NodeCategory::Computation),
            common::plain_node("b", NodeCategory::Return),
        ],
        vec![("a", "b", EdgeFamily::Data)],
        "borrow",
    );
    function.matching_profile = if data {
        MatchingProfile::DataV1
    } else {
        MatchingProfile::ScalarV1
    };
    function.inputs.push(InterfaceElement {
        role: "parameter".into(),
        normalized_type: r#"{"kind":"shared","value":{"kind":"scalar","value":"u32"}}"#.into(),
        variadic: false,
    });
    function
}

#[test]
fn data_matching_is_registered_separately_and_reuses_exact_scoring() {
    let data = function("data", true);
    let report = evaluate_all(
        &[data.clone(), function("second", true)],
        &CcgraphConfig::default(),
    );
    assert_eq!(report.profile_id, "rstyle.ccgraph.data");
    assert!(report.functions.iter().all(|function| function.eligible));
    assert_eq!(report.pairs.len(), 1);
    assert_eq!(report.pairs[0].decision, rstyle_ccgraph::Decision::Accepted);
    let scalar = evaluate_all(&[function("scalar", false)], &CcgraphConfig::default());
    assert_eq!(scalar.profile_id, "compat.ccgraph.strict");
    assert!(!scalar.functions[0].eligible);
    let mut legacy = function("legacy", false);
    legacy.inputs.clear();
    let mixed = evaluate_all(&[legacy, data], &CcgraphConfig::default());
    assert!(mixed.functions.iter().all(|function| function.eligible));
    assert_ne!(mixed.pairs[0].decision, rstyle_ccgraph::Decision::Accepted);
}

#[test]
fn selected_matching_identity_is_explicit_for_empty_and_mismatched_scopes() {
    let config = CcgraphConfig::default();
    let report =
        rstyle_ccgraph::pipeline::evaluate_selected(&[], &config, None, MatchingProfile::DataV1);
    assert_eq!(report.profile_id, "rstyle.ccgraph.data");
    assert!(!report.incomplete);
    let mismatch = rstyle_ccgraph::pipeline::evaluate_selected(
        &[function("data", true)],
        &config,
        None,
        MatchingProfile::ScalarV1,
    );
    assert!(mismatch.incomplete);
    assert_eq!(mismatch.pairs.len(), 0);
}

#[test]
fn data_pair_resource_limit_is_incomplete_execution() {
    let report = rstyle_ccgraph::pipeline::evaluate_selected(
        &[function("a", true), function("b", true)],
        &CcgraphConfig::default(),
        Some(0),
        MatchingProfile::DataV1,
    );
    assert!(report.incomplete);
    assert!(report.pairs.is_empty());
    assert!(report.diagnostics[0].contains("requires 1 pairs"));
}
