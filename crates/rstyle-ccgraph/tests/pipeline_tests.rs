//! Gate, boundary, direction, multiplicity, ordering, and variant tests.
//!
//! These cover the milestone's required behaviors beyond the vendored stage
//! fixtures: malformed inputs, unknown evidence, inclusive boundaries,
//! direction and multiplicity sensitivity, ID renaming, collection order,
//! scoring variants, and explicit stage traces.

mod common;

use common::*;
use rstyle_ccgraph::*;
use rstyle_pdg::{EdgeFamily, NodeCategory};

fn chain(key: &str, size: usize, name: &str) -> ImportedFunction {
    let nodes = (0..size)
        .map(|i| plain_node(&format!("n{i}"), NodeCategory::Computation))
        .collect();
    let edges = (0..size.saturating_sub(1))
        .map(|i| (format!("n{i}"), format!("n{}", i + 1)))
        .collect::<Vec<_>>();
    let edge_refs: Vec<(&str, &str, EdgeFamily)> = edges
        .iter()
        .map(|(s, t)| (s.as_str(), t.as_str(), EdgeFamily::Data))
        .collect();
    stage_function(key, nodes, edge_refs, name)
}

fn two_node(key: &str, family: EdgeFamily, name: &str) -> ImportedFunction {
    stage_function(
        key,
        vec![
            plain_node("x", NodeCategory::Declaration),
            plain_node("y", NodeCategory::Assignment),
        ],
        vec![("x", "y", family)],
        name,
    )
}

fn accepted(report: &RunReport, first: &str, second: &str) -> bool {
    report
        .pairs
        .iter()
        .find(|p| p.first == first && p.second == second)
        .map(|p| p.decision == Decision::Accepted)
        .unwrap_or(false)
}

fn stage_state(report: &RunReport, first: &str, second: &str, stage: StageName) -> StageOutcome {
    report
        .pairs
        .iter()
        .find(|p| p.first == first && p.second == second)
        .and_then(|p| p.stages.iter().find(|s| s.stage == stage))
        .map(|s| s.outcome.clone())
        .expect("stage exists")
}

fn is_not_evaluated(outcome: &StageOutcome) -> bool {
    matches!(outcome, StageOutcome::NotEvaluated)
}

fn dec(lexeme: &str) -> Frac {
    Frac::parse_decimal(lexeme).expect("decimal threshold")
}

/// Every gate passes trivially; only size ratio can still reject.
fn open_config() -> CcgraphConfig {
    CcgraphConfig {
        minimum_size_ratio: CcgraphConfig::default().minimum_size_ratio,
        numerical_threshold: dec("0"),
        string_threshold: dec("0"),
        final_threshold: dec("0"),
        scoring: Scoring::SmallerGraphOverlap,
    }
}

#[test]
fn identical_functions_are_accepted_with_full_traces() {
    let report = evaluate_all(
        &[chain("a", 2, "same"), chain("b", 2, "same")],
        &CcgraphConfig::default(),
    );
    assert!(accepted(&report, "a", "b"));
    assert!(!report.incomplete);
    let pair = report.pairs.iter().find(|p| p.first == "a").expect("pair");
    assert!(
        pair.stages
            .iter()
            .all(|s| matches!(s.outcome, StageOutcome::Passed))
    );
    assert!(pair.displayed_score.is_some());
    assert_eq!(report.functions.len(), 2);
}

#[test]
fn size_ratio_boundary_is_inclusive_and_later_stages_skip() {
    let config = open_config();
    let report = evaluate_all(&[chain("a", 4, "same"), chain("b", 5, "same")], &config);
    assert!(accepted(&report, "a", "b"));
    let report = evaluate_all(&[chain("a", 4, "same"), chain("c", 6, "same")], &config);
    assert!(!accepted(&report, "a", "c"));
    assert!(matches!(
        stage_state(&report, "a", "c", StageName::SizeRatio),
        StageOutcome::Rejected { .. }
    ));
    assert!(is_not_evaluated(&stage_state(
        &report,
        "a",
        "c",
        StageName::Numerical
    )));
    assert!(is_not_evaluated(&stage_state(
        &report,
        "a",
        "c",
        StageName::Strings
    )));
    assert!(is_not_evaluated(&stage_state(
        &report,
        "a",
        "c",
        StageName::WlFinal
    )));
}

#[test]
fn zero_ratio_disables_ratio_rejection_but_not_nonempty() {
    let config = CcgraphConfig {
        minimum_size_ratio: dec("0"),
        numerical_threshold: dec("0"),
        string_threshold: dec("0"),
        final_threshold: dec("0"),
        scoring: Scoring::SmallerGraphOverlap,
    };
    let report = evaluate_all(&[chain("a", 1, "same"), chain("b", 9, "same")], &config);
    assert!(accepted(&report, "a", "b"));
    let empty = ImportedFunction {
        nodes: Vec::new(),
        edges: Vec::new(),
        ..chain("e", 1, "same")
    };
    let report = evaluate_all(&[empty, chain("b", 9, "same")], &config);
    assert!(!accepted(&report, "b", "e"));
    assert!(matches!(
        stage_state(&report, "b", "e", StageName::Nonempty),
        StageOutcome::Rejected { .. }
    ));
}

#[test]
fn reversed_direction_fails_only_the_final_gate() {
    let config = CcgraphConfig {
        string_threshold: dec("0"),
        ..CcgraphConfig::default()
    };
    let report = evaluate_all(
        &[
            stage_function(
                "a",
                vec![
                    plain_node("x", NodeCategory::Declaration),
                    plain_node("y", NodeCategory::Assignment),
                ],
                vec![("x", "y", EdgeFamily::Data)],
                "same",
            ),
            stage_function(
                "b",
                vec![
                    plain_node("x", NodeCategory::Declaration),
                    plain_node("y", NodeCategory::Assignment),
                ],
                vec![("y", "x", EdgeFamily::Data)],
                "same",
            ),
        ],
        &config,
    );
    assert!(matches!(
        stage_state(&report, "a", "b", StageName::Numerical),
        StageOutcome::Passed
    ));
    assert!(matches!(
        stage_state(&report, "a", "b", StageName::WlFinal),
        StageOutcome::Rejected { .. }
    ));
    assert!(!accepted(&report, "a", "b"));
}

#[test]
fn changed_family_and_added_multiplicity_fail_the_final_gate() {
    let config = CcgraphConfig {
        numerical_threshold: dec("0"),
        string_threshold: dec("0"),
        ..CcgraphConfig::default()
    };
    let base = two_node("a", EdgeFamily::Data, "same");
    let other_family = two_node("b", EdgeFamily::Control, "same");
    let mut doubled = two_node("c", EdgeFamily::Data, "same");
    doubled.edges.push(doubled.edges[0].clone());
    let report = evaluate_all(&[base.clone(), other_family, doubled], &config);
    assert!(!accepted(&report, "a", "b"));
    assert!(!accepted(&report, "a", "c"));
    assert!(matches!(
        stage_state(&report, "a", "b", StageName::WlFinal),
        StageOutcome::Rejected { .. }
    ));
    assert!(matches!(
        stage_state(&report, "a", "c", StageName::WlFinal),
        StageOutcome::Rejected { .. }
    ));
}

#[test]
fn unknown_reference_count_is_ineligible_with_skipped_stages() {
    let mut unknown = chain("u", 2, "same");
    unknown.reference_count = None;
    let report = evaluate_all(&[unknown, chain("b", 2, "same")], &CcgraphConfig::default());
    assert!(!accepted(&report, "b", "u"));
    assert!(matches!(
        stage_state(&report, "b", "u", StageName::Validation),
        StageOutcome::Ineligible { .. }
    ));
    for stage in [
        StageName::Projection,
        StageName::Nonempty,
        StageName::Numerical,
        StageName::Strings,
        StageName::WlFinal,
    ] {
        assert!(is_not_evaluated(&stage_state(&report, "b", "u", stage)));
    }
    let record = report
        .functions
        .iter()
        .find(|f| f.key == "u")
        .expect("function");
    assert!(!record.eligible);
}

#[test]
fn reference_count_mismatch_blocks_projection() {
    let mut mismatch = chain("m", 2, "same");
    mismatch.reference_count = Some(99);
    let report = evaluate_all(
        &[mismatch, chain("b", 2, "same")],
        &CcgraphConfig::default(),
    );
    assert!(matches!(
        stage_state(&report, "b", "m", StageName::Projection),
        StageOutcome::Ineligible { .. }
    ));
}

#[test]
fn duplicate_keys_are_rejected_before_enumeration() {
    let report = evaluate_all(
        &[
            chain("a", 1, "same"),
            chain("a", 1, "same"),
            chain("b", 1, "same"),
        ],
        &CcgraphConfig::default(),
    );
    assert!(report.incomplete);
    assert!(report.pairs.is_empty());
    assert!(report.diagnostics.iter().any(|d| d.contains("duplicate")));
}

#[test]
fn pair_enumeration_is_exhaustive_ordered_and_stable() {
    let funcs = vec![
        chain("c", 1, "same"),
        chain("a", 1, "same"),
        chain("b", 1, "same"),
    ];
    let first = evaluate_all(&funcs, &CcgraphConfig::default());
    let keys: Vec<(&str, &str)> = first
        .pairs
        .iter()
        .map(|p| (p.first.as_str(), p.second.as_str()))
        .collect();
    assert_eq!(keys, vec![("a", "b"), ("a", "c"), ("b", "c")]);
    let again = evaluate_all(&funcs, &CcgraphConfig::default());
    assert_eq!(first, again);
}

#[test]
fn experimental_cosine_uses_the_same_depths_and_weights() {
    let config = CcgraphConfig {
        scoring: Scoring::CrossProductCosine,
        ..CcgraphConfig::default()
    };
    let report = evaluate_all(&[chain("a", 2, "same"), chain("b", 2, "same")], &config);
    assert!(accepted(&report, "a", "b"));
    assert_eq!(report.scoring, "cross_product_cosine");
    let pair = &report.pairs[0];
    let wl = pair
        .stages
        .iter()
        .find(|s| s.stage == StageName::WlFinal)
        .expect("wl");
    assert!(wl.detail.contains_key("kab"));
    assert!(wl.detail.contains_key("kaa"));
    assert!(wl.detail.contains_key("kbb"));
    assert!(wl.detail.contains_key("depth"));
    assert!(wl.detail.contains_key("weights"));
}

#[test]
fn configuration_errors_are_explicit() {
    assert!(parse_config(&serde_json::json!({"unknown_key": 1})).is_err());
    assert!(parse_config(&serde_json::json!({"scoring": "overlap"})).is_err());
    assert!(parse_config(&serde_json::json!({"final_threshold": "2"})).is_err());
    assert!(parse_config(&serde_json::json!({"final_threshold": "much"})).is_err());
    assert!(parse_config(&serde_json::json!({"final_threshold": [1]})).is_err());
    let config = parse_config(
        &serde_json::json!({"scoring": "cross_product_cosine", "minimum_size_ratio": "0.5"}),
    )
    .expect("valid");
    assert_eq!(config.scoring, Scoring::CrossProductCosine);
    assert_eq!(
        config.minimum_size_ratio,
        Frac::parse_decimal("0.5").expect("t")
    );
}

#[test]
fn native_width_mismatch_is_pair_ineligible() {
    let mut wide_a = chain("a", 1, "same");
    wide_a.native_width_involved = true;
    wide_a.target_pointer_width = Some(64);
    let mut wide_b = chain("b", 1, "same");
    wide_b.native_width_involved = true;
    wide_b.target_pointer_width = Some(32);
    let report = evaluate_all(&[wide_a, wide_b], &CcgraphConfig::default());
    assert!(matches!(
        stage_state(&report, "a", "b", StageName::Validation),
        StageOutcome::Ineligible { .. }
    ));
    let mut same = chain("c", 1, "same");
    same.native_width_involved = true;
    same.target_pointer_width = Some(64);
    let mut same2 = chain("d", 1, "same");
    same2.native_width_involved = true;
    same2.target_pointer_width = Some(64);
    let report = evaluate_all(&[same, same2], &CcgraphConfig::default());
    assert!(accepted(&report, "c", "d"));
}

#[test]
fn unsupported_normalized_types_and_unknown_names_are_ineligible() {
    let mut typed = chain("t", 1, "same");
    typed.inputs[0].normalized_type = "f64".to_string();
    let mut nameless = chain("n", 1, "same");
    nameless.name = None;
    let report = evaluate_all(
        &[typed, nameless, chain("b", 1, "same")],
        &CcgraphConfig::default(),
    );
    assert!(
        !report
            .functions
            .iter()
            .find(|f| f.key == "t")
            .expect("t")
            .eligible
    );
    assert!(
        !report
            .functions
            .iter()
            .find(|f| f.key == "n")
            .expect("n")
            .eligible
    );
    assert!(!accepted(&report, "b", "n"));
    assert!(!accepted(&report, "b", "t"));
}

#[test]
fn exact_final_boundary_passes_without_float_rounding() {
    let config = CcgraphConfig {
        numerical_threshold: dec("0"),
        string_threshold: dec("0"),
        final_threshold: dec("0.9"),
        ..CcgraphConfig::default()
    };
    let report = evaluate_all(
        &[
            two_node("a", EdgeFamily::Data, "same"),
            two_node("b", EdgeFamily::Data, "same"),
        ],
        &config,
    );
    assert!(accepted(&report, "a", "b"));
    let pair = &report.pairs[0];
    let wl = pair
        .stages
        .iter()
        .find(|s| s.stage == StageName::WlFinal)
        .expect("wl");
    assert_eq!(wl.detail.get("score").map(String::as_str), Some("1/1"));
}

#[test]
fn self_loop_zero_depth_scores_only_initial_labels() {
    let config = CcgraphConfig {
        numerical_threshold: dec("0"),
        string_threshold: dec("0"),
        ..CcgraphConfig::default()
    };
    let mut looped = stage_function(
        "l",
        vec![plain_node("x", NodeCategory::Computation)],
        vec![],
        "same",
    );
    looped.edges.push(ImportedEdge {
        source: "x".to_string(),
        target: "x".to_string(),
        family: EdgeFamily::Data,
    });
    let bare = stage_function(
        "s",
        vec![plain_node("x", NodeCategory::Computation)],
        vec![],
        "same",
    );
    let report = evaluate_all(&[looped, bare], &config);
    let pair = &report.pairs[0];
    let wl = pair
        .stages
        .iter()
        .find(|s| s.stage == StageName::WlFinal)
        .expect("wl");
    assert_eq!(wl.detail.get("depth").map(String::as_str), Some("0"));
    assert_eq!(wl.detail.get("score").map(String::as_str), Some("1/1"));
    assert!(accepted(&report, "l", "s"));
}
