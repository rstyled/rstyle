//! Slice-4 regression: BFS discovery, exact threshold lexemes, arithmetic
//! error propagation, checked Frac normalization, per-edge trace.

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

#[test]
fn long_parallel_chain_diameter_is_bounded_and_linear() {
    // 120 vertices, 8 parallel data edges per step. A naive BFS that
    // enqueues one entry per occurrence would fan out exponentially;
    // discover-once keeps it linear with diameter 119.
    let size = 120usize;
    let fan = 8usize;
    let nodes: Vec<ImportedNode> = (0..size)
        .map(|i| plain_node(&format!("n{i}"), NodeCategory::Computation))
        .collect();
    let mut func = stage_function("chain", nodes, vec![], "same");
    for i in 0..size - 1 {
        for k in 0..fan {
            func.edges.push(ImportedEdge {
                source: format!("n{i}"),
                target: format!("n{}", i + 1),
                family: EdgeFamily::Data,
                origin: format!("chain-{i}-{k}"),
            });
        }
    }
    let made = project(&func).expect("projects");
    assert_eq!(made.graph.edges.len(), (size - 1) * fan);
    assert_eq!(diameter(&made.graph), size - 1);
    // Per-occurrence trace keeps every parallel edge with its origin.
    assert_eq!(made.trace.edges.len(), (size - 1) * fan);
    assert!(made.trace.edges.iter().all(|e| matches!(
        e.disposition,
        rstyle_ccgraph::EdgeDisposition::Retained { .. }
    )));
}

#[test]
fn exact_decimal_lexeme_survives_json_number_ingestion() {
    // 0.50000000000000001 as a JSON number must not collapse to 0.5 via f64.
    let value: serde_json::Value =
        serde_json::from_str(r#"{"numerical_threshold": 0.50000000000000001}"#)
            .expect("parse");
    let config = parse_config(&value).expect("config");
    assert_eq!(
        config.numerical_threshold,
        Frac::parse_decimal("0.50000000000000001").expect("exact")
    );
    assert_ne!(
        config.numerical_threshold,
        Frac::parse_decimal("0.5").expect("half")
    );
}

#[test]
fn out_of_range_high_precision_threshold_is_rejected() {
    let value: serde_json::Value =
        serde_json::from_str(r#"{"final_threshold": 1.00000000000000001}"#).expect("parse");
    assert!(
        parse_config(&value).is_err(),
        "1.00000000000000001 exceeds [0,1]"
    );
}

#[test]
fn string_threshold_number_lexeme_is_exact() {
    let value: serde_json::Value =
        serde_json::from_str(r#"{"string_threshold": 0.50000000000000001}"#).expect("parse");
    let config = parse_config(&value).expect("config");
    assert_eq!(
        config.string_threshold,
        Frac::parse_decimal("0.50000000000000001").expect("exact")
    );
}

#[test]
fn checked_signed_normalization_does_not_wrap() {
    // i128::MIN negated would wrap; normalization must report overflow.
    assert!(Frac::new(i128::MIN, -1).is_err());
    assert!(Frac::new(i128::MIN, i128::MIN).is_ok());
    assert!(Frac::new(2, -4).expect("neg").numerator() == -1);
    // Zero denominator stays an error, never a wrapped value.
    assert!(Frac::new(1, 0).is_err());
}

#[test]
fn arithmetic_failure_is_error_and_marks_run_incomplete() {
    // A 38-digit threshold makes the size-ratio cross-multiplication
    // overflow i128 even for tiny graphs. That overflow must surface as a
    // stage Error (not a Rejected pair) and mark the run incomplete: an
    // ordinary arithmetic failure must never look like a complete run of
    // only rejected pairs.
    let big = "0.99999999999999999999999999999999999999";
    assert_eq!(big.split('.').nth(1).expect("dp").len(), 38);
    let threshold = Frac::parse_decimal(big).expect("threshold parses");
    let config = CcgraphConfig {
        minimum_size_ratio: threshold,
        numerical_threshold: Frac::parse_decimal("0").expect("t"),
        string_threshold: Frac::parse_decimal("0").expect("t"),
        final_threshold: Frac::parse_decimal("0").expect("t"),
        scoring: Scoring::SmallerGraphOverlap,
    };
    // 2 vs 3 nodes: 2*10^38 overflows i128, forcing the size-ratio
    // comparison to fail arithmetically.
    let report = evaluate_all(
        &[chain("a", 2, "same"), chain("b", 3, "same")],
        &config,
    );
    let pair = report
        .pairs
        .iter()
        .find(|p| p.first == "a")
        .expect("pair");
    let ratio = pair
        .stages
        .iter()
        .find(|s| s.stage == StageName::SizeRatio)
        .expect("size_ratio");
    assert!(
        matches!(ratio.outcome, StageOutcome::Error { .. }),
        "overflow is Error, got {:?}",
        ratio.outcome
    );
    assert!(
        report.incomplete,
        "arithmetic failure must mark the run incomplete"
    );
    assert!(
        !report.pairs.iter().all(|p| p.decision == Decision::Rejected)
            || report.incomplete,
        "a complete run must not contain only rejected pairs after failure"
    );
}

#[test]
fn zero_norm_vectors_are_a_contract_diagnostic() {
    // Direct unit check: an all-zero numerical vector has no cosine.
    let zero = [0u64; 9];
    let one = [0, 0, 0, 1, 0, 0, 0, 0, 0];
    assert!(cosine_parts(&zero, &one).is_err());
    assert!(cosine_parts(&zero, &zero).is_err());
}

#[test]
fn projection_trace_records_each_edge_occurrence_and_origin() {
    let func = stage_function(
        "t",
        vec![
            plain_node("a", NodeCategory::Declaration),
            plain_node("b", NodeCategory::Assignment),
        ],
        vec![],
        "same",
    );
    let mut doubled = func;
    for k in 0..3 {
        doubled.edges.push(ImportedEdge {
            source: "a".to_string(),
            target: "b".to_string(),
            family: EdgeFamily::Data,
            origin: format!("occ-{k}"),
        });
    }
    let made = project(&doubled).expect("projects");
    assert_eq!(made.graph.edges.len(), 3);
    assert_eq!(made.trace.edges.len(), 3);
    let origins: Vec<&str> = made
        .trace
        .edges
        .iter()
        .map(|e| e.origin.as_str())
        .collect();
    assert_eq!(origins, vec!["occ-0", "occ-1", "occ-2"]);
}
