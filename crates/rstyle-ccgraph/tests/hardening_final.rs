//! Final hardening: rename/reorder invariance, equality partitions,
//! production comparator boundaries, and adversarial cases.

use rstyle_ccgraph::*;
use rstyle_pdg::{EdgeFamily, NodeCategory};
use std::collections::BTreeMap;

mod common;

use common::*;

#[test]
fn renamed_and_reordered_inputs_match_baseline() {
    let base_a = chain("a", 4, "same");
    let base_b = chain("b", 4, "same");
    let baseline = evaluate_all(&[base_a.clone(), base_b.clone()], &CcgraphConfig::default());
    let baseline_pair = baseline
        .pairs
        .iter()
        .find(|pair| pair.first == "a")
        .expect("pair");

    let renamed_a = rename_function(&base_a, "a", "q");
    let renamed_b = rename_function(&base_b, "b", "z");
    let reordered_a = reorder_edges(&renamed_a);
    let report = evaluate_all(&[reordered_a, renamed_b], &CcgraphConfig::default());
    let pair = report
        .pairs
        .iter()
        .find(|pair| pair.first == "q")
        .expect("renamed pair");

    assert_eq!(pair.decision, baseline_pair.decision);
    assert_eq!(pair.displayed_score, baseline_pair.displayed_score);
    assert_eq!(
        stage_outcomes(pair),
        stage_outcomes(baseline_pair),
        "rename/reorder must not change stage results"
    );
}

fn stage_outcomes(pair: &PairRecord) -> Vec<(StageName, String)> {
    pair.stages
        .iter()
        .map(|stage| (stage.stage, format!("{:?}", stage.outcome)))
        .collect()
}

fn rename_function(func: &ImportedFunction, from: &str, to: &str) -> ImportedFunction {
    let rename = |id: &str| id.replace(from, to);
    let mut renamed = func.clone();

    renamed.key = rename(&func.key);

    for node in &mut renamed.nodes {
        node.id = rename(&node.id);
        node.read_symbols = node
            .read_symbols
            .iter()
            .map(|symbol| rename(symbol))
            .collect();
    }

    for edge in &mut renamed.edges {
        edge.source = rename(&edge.source);
        edge.target = rename(&edge.target);
    }

    renamed
}

fn reorder_edges(func: &ImportedFunction) -> ImportedFunction {
    let mut reordered = func.clone();
    reordered.edges.reverse();
    reordered.nodes.reverse();
    reordered
}

fn chain(key: &str, size: usize, name: &str) -> ImportedFunction {
    let nodes = (0..size)
        .map(|index| plain_node(&format!("n{index}"), NodeCategory::Computation))
        .collect();
    let edges = (0..size.saturating_sub(1))
        .map(|index| (format!("n{index}"), format!("n{}", index + 1)))
        .collect::<Vec<_>>();
    let edge_refs: Vec<(&str, &str, EdgeFamily)> = edges
        .iter()
        .map(|(source, target)| (source.as_str(), target.as_str(), EdgeFamily::Data))
        .collect();

    stage_function(key, nodes, edge_refs, name)
}

#[test]
fn wl_equality_partitions_survive_renaming() {
    let left = chain("a", 3, "same");
    let right = chain("b", 3, "same");
    let renamed = rename_function(&right, "b", "q");
    let depth = 2;

    let plain = refine_pair(&left, &right, depth);
    let swapped = refine_pair(&left, &renamed, depth);

    assert_eq!(plain, swapped, "partitions must agree under renaming");
}

fn refine_pair(first: &ImportedFunction, second: &ImportedFunction, depth: usize) -> Vec<Vec<u64>> {
    let graph_a = project(first).expect("project").graph;
    let graph_b = project(second).expect("project").graph;
    let refinement = refine(&graph_a, &graph_b, depth);
    let rounds: Vec<(Histogram, Histogram)> = refinement
        .rounds
        .iter()
        .map(|(a, b)| (labeled(a), labeled(b)))
        .collect();
    let scores = score_rounds(&rounds, &weights(depth))
        .expect("scores")
        .overlap_per_round
        .clone();

    let _ = scores;
    partition_sizes(&refinement.rounds)
}

fn labeled(hist: &BTreeMap<u64, u64>) -> Histogram {
    hist.iter()
        .map(|(label, count)| (format!("L{label}"), *count))
        .collect()
}

type RoundPair = (BTreeMap<u64, u64>, BTreeMap<u64, u64>);

/// Sorted per-round class sizes for both graphs: renaming labels must not
/// change partition structure.
fn partition_sizes(rounds: &[RoundPair]) -> Vec<Vec<u64>> {
    rounds
        .iter()
        .flat_map(|(a, b)| [sorted_counts(a), sorted_counts(b)])
        .collect()
}

fn sorted_counts(hist: &BTreeMap<u64, u64>) -> Vec<u64> {
    let mut counts: Vec<u64> = hist.values().copied().collect();
    counts.sort();
    counts
}

#[test]
fn production_numerical_cosine_boundary_is_exact() {
    let threshold = Frac::parse_decimal("0.9").expect("t");
    // Cosine exactly 0.9: norms 100/100, dot 90.
    let a = [10, 0, 0, 0, 0, 0, 0, 0, 0];
    let exact = [9, 4, 1, 1, 1, 0, 0, 0, 0];
    let below = [8, 6, 0, 0, 0, 0, 0, 0, 0];

    let exact_parts = cosine_parts(&a, &exact).expect("parts");
    assert_eq!(exact_parts.dot, 90);
    assert_eq!(exact_parts.norm_a_squared, 100);
    assert_eq!(exact_parts.norm_b_squared, 100);
    assert!(cosine_passes(exact_parts, threshold).expect("cmp"));

    let below_parts = cosine_parts(&a, &below).expect("parts");
    assert_eq!(below_parts.dot, 80);
    assert!(!cosine_passes(below_parts, threshold).expect("cmp"));
}

#[test]
fn production_wl_cosine_boundary_is_exact() {
    use rstyle_ccgraph::scoring::{CosineDetail, cosine_passes as wl_passes};

    let threshold = Frac::parse_decimal("0.9").expect("t");
    let exact = CosineDetail {
        kab: 90,
        kaa: 100,
        kbb: 100,
    };
    assert!(wl_passes(&exact, threshold).expect("cmp"));

    let below = CosineDetail {
        kab: 89,
        kaa: 100,
        kbb: 100,
    };
    assert!(!wl_passes(&below, threshold).expect("cmp"));
}

#[test]
fn merge_without_value_sites_is_excluded_not_kept() {
    let mut func = stage_function(
        "m",
        vec![
            plain_node("x", NodeCategory::Computation),
            ImportedNode {
                value_site_reachable: false,
                reachable: false,
                ..plain_node("g", NodeCategory::Merge)
            },
        ],
        vec![("x", "g", EdgeFamily::Data)],
        "same",
    );
    func.reference_count = Some(0);

    let made = project(&func).expect("projects");
    assert!(!made.graph.nodes.iter().any(|node| node.id == "g"));
    assert!(
        made.trace.nodes.iter().any(|(id, _)| id == "g"),
        "removal must be recorded"
    );
}

#[test]
fn partial_operation_map_is_rejected() {
    // One triple is not enough when the CFG has two operations: coverage
    // must fail rather than silently certify the mapped half.
    let cfg = two_op_cfg();
    let pdg = ["n0", "n1"].iter().map(|s| s.to_string()).collect();
    let partial = vec![rstyle_pdg::OperationMapEntry {
        cfg_node_id: "b1".to_string(),
        cfg_operation_id: "op0".to_string(),
        pdg_node_id: "n0".to_string(),
    }];

    assert!(
        rstyle_pdg::validate_correspondence(&partial, &cfg, &pdg)
            .iter()
            .any(|issue| issue.code == "cfg.operation-map-incomplete")
    );
}

fn test_span() -> rstyle_pdg::Span {
    rstyle_pdg::Span {
        source_id: "s0".to_string(),
        start: 0,
        end: 1,
    }
}

fn test_block(id: &str, op: &str) -> rstyle_pdg::CfgNode {
    rstyle_pdg::CfgNode {
        id: id.to_string(),
        kind: rstyle_pdg::CfgNodeKind::Block,
        origin: "test".to_string(),
        exit_kind: None,
        operations: vec![rstyle_pdg::CfgOperation {
            id: op.to_string(),
            original_kind: "k".to_string(),
            text: "t".to_string(),
            source_span: test_span(),
        }],
    }
}

fn test_edge(
    id: &str,
    source: &str,
    target: &str,
    kind: rstyle_pdg::CfgEdgeKind,
) -> rstyle_pdg::CfgEdge {
    rstyle_pdg::CfgEdge {
        id: id.to_string(),
        source: source.to_string(),
        target: target.to_string(),
        kind,
        case_value: None,
        loop_role: None,
        reason: None,
    }
}

fn two_op_producer() -> rstyle_pdg::CfgProducer {
    rstyle_pdg::CfgProducer {
        name: "t".to_string(),
        version: "0".to_string(),
        language: "Rust".to_string(),
        language_version: rstyle_pdg::StringFact::known("1.0"),
        configuration: BTreeMap::new(),
    }
}

fn two_op_function() -> rstyle_pdg::CfgFunction {
    rstyle_pdg::CfgFunction {
        id: "f".to_string(),
        name: rstyle_pdg::StringFact::known("alpha"),
        source_span: test_span(),
    }
}

fn two_op_cfg() -> rstyle_pdg::CfgDocument {
    rstyle_pdg::CfgDocument {
        schema_version: rstyle_pdg::CFG_SCHEMA_VERSION.to_string(),
        profile: rstyle_pdg::r0_profile(),
        producer: two_op_producer(),
        sources: vec![rstyle_pdg::CfgSource {
            id: "s0".to_string(),
            path: "a.rs".to_string(),
            content_identity: rstyle_pdg::StringFact::missing(
                rstyle_pdg::FactStatus::Unknown,
                "hand-authored",
            ),
            content: None,
        }],
        function: two_op_function(),
        entry_node_id: "E".to_string(),
        exit_node_ids: vec!["X".to_string()],
        nodes: vec![
            rstyle_pdg::CfgNode {
                id: "E".to_string(),
                kind: rstyle_pdg::CfgNodeKind::Entry,
                origin: "entry".to_string(),
                exit_kind: None,
                operations: Vec::new(),
            },
            test_block("b1", "op0"),
            test_block("b2", "op1"),
            rstyle_pdg::CfgNode {
                id: "X".to_string(),
                kind: rstyle_pdg::CfgNodeKind::Exit,
                origin: "exit".to_string(),
                exit_kind: Some(rstyle_pdg::CfgExitKind::Normal),
                operations: Vec::new(),
            },
        ],
        edges: vec![
            test_edge("c0", "E", "b1", rstyle_pdg::CfgEdgeKind::Normal),
            test_edge("c1", "b1", "b2", rstyle_pdg::CfgEdgeKind::Normal),
            test_edge("c2", "b2", "X", rstyle_pdg::CfgEdgeKind::Return),
        ],
        extraction_evidence: rstyle_pdg::CfgExtractionEvidence {
            status: rstyle_pdg::ExtractionStatus::Complete,
            limitations: Vec::new(),
        },
        extensions: BTreeMap::new(),
    }
}

#[test]
fn comparison_overflow_is_error_never_acceptance() {
    use rstyle_ccgraph::scoring::{Histogram, score_rounds};

    // Absurd histogram counts overflow the weighted kernel: scoring must
    // report an error, and no acceptance decision may be built on it.
    let huge: Histogram = BTreeMap::from([("L0".to_string(), u64::MAX)]);
    let rounds = vec![(huge.clone(), huge)];
    let weights = vec![u64::MAX, u64::MAX];

    assert!(score_rounds(&rounds, &weights).is_err());
}
