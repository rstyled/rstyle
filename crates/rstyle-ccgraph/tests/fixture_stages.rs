//! Stage conformance against the vendored matching-fixture snapshot.
//!
//! The compact fixtures are hand-worked stage expectations, not emitted
//! graphs. These tests materialize typed inputs and check every stage:
//! projection, interface serialization, Jaro-Winkler, diameter, depth,
//! WL refinement, both scorings, and inclusive boundaries.

mod stage;

use rstyle_ccgraph::*;
use rstyle_pdg::EdgeFamily;
use stage::*;
use std::collections::{BTreeMap, BTreeSet};

fn matching() -> serde_json::Value {
    load_vendor("fixtures/ccgraph-matching.json")
}

fn profile() -> serde_json::Value {
    load_vendor("ccgraph-profile.json")
}

fn fraction(pair: &serde_json::Value) -> Frac {
    let nums = pair.as_array().expect("fraction pair");
    Frac::new(
        nums[0].as_i64().expect("num") as i128,
        nums[1].as_i64().expect("den") as i128,
    )
    .expect("valid fraction")
}

#[test]
fn profile_machine_settings_match_implementation_defaults() {
    let machine = profile();
    assert_eq!(
        machine["profile"],
        serde_json::json!({"id": "compat.ccgraph.strict", "version": "0.1.0"})
    );
    let defaults = CcgraphConfig::default();
    assert_eq!(
        frac_of(&machine["defaults"]["minimum_size_ratio"]),
        defaults.minimum_size_ratio
    );
    assert_eq!(
        frac_of(&machine["defaults"]["numerical_threshold"]),
        defaults.numerical_threshold
    );
    assert_eq!(
        frac_of(&machine["defaults"]["string_threshold"]),
        defaults.string_threshold
    );
    assert_eq!(
        frac_of(&machine["defaults"]["final_threshold"]),
        defaults.final_threshold
    );
    assert_eq!(machine["defaults"]["scoring"], "smaller_graph_overlap");
    assert_eq!(
        machine["scoring"]["experimental"],
        serde_json::json!(["cross_product_cosine"])
    );
    assert_eq!(machine["wl"]["round_weight"], "h+1-r");
    assert!(machine["wl"]["refinement_cap"].is_null());
    assert!(machine["wl"]["graph_size_fallback"].is_null());
    assert_eq!(machine["gates"]["composition"], "sequential-AND");
    assert!(!machine["gates"]["preliminary_wl_gate"].as_bool().unwrap());
    let mapping = machine["category_mapping"].as_object().expect("mapping");
    assert_eq!(mapping["declaration"], "DECL");
    assert_eq!(mapping["call"], "STATE");
    assert_eq!(mapping["read"], "OTHER");
}

fn frac_of(value: &serde_json::Value) -> Frac {
    let lexeme = match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Number(number) => number.to_string(),
        _ => panic!("threshold must be a decimal"),
    };
    Frac::parse_decimal(&lexeme).expect("valid threshold")
}

#[test]
fn projection_cases_retain_values_and_drop_only_boundaries() {
    for case in matching()["projection_cases"].as_array().expect("cases") {
        let func = projection_input(case);
        let made = project(&func).expect("stage projection succeeds");
        assert_retained(&made, case);
        assert_edges(&made, case);
        let features = numerical_features(&made.graph, made.facts.reference_count);
        assert_eq!(
            features.to_vec(),
            json_numbers(&case["expected"]["features"]),
            "{}",
            case["id"]
        );
    }
}

fn projection_input(case: &serde_json::Value) -> ImportedFunction {
    ImportedFunction {
        matching_profile: rstyle_ccgraph::MatchingProfile::ScalarV1,
        extraction_context: None,
        key: "projection-stage".to_string(),
        nodes: projection_nodes(case),
        edges: projection_edges(case),
        inputs: Vec::new(),
        outputs: Vec::new(),
        name: Some("stage".to_string()),
        reference_count: Some(1),
        target_pointer_width: Some(64),
        native_width_involved: false,
    }
}

fn projection_nodes(case: &serde_json::Value) -> Vec<ImportedNode> {
    case["input"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .map(|node| ImportedNode {
            id: node["id"].as_str().expect("id").to_string(),
            category: parse_category(node["category"].as_str().expect("category")),
            administrative: flag(node, "administrative", false),
            reachable: flag(node, "reachable", true),
            defines_binding: false,
            reads_or_writes: false,
            value_site_reachable: true,
            read_symbols: symbols(node),
            registered_meaning: None,
        })
        .collect()
}

fn flag(node: &serde_json::Value, field: &str, default: bool) -> bool {
    node.get(field).and_then(|v| v.as_bool()).unwrap_or(default)
}

fn symbols(node: &serde_json::Value) -> Vec<String> {
    node.get("read_symbols")
        .and_then(|v| v.as_array())
        .map(|list| {
            list.iter()
                .map(|s| s.as_str().expect("symbol").to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn projection_edges(case: &serde_json::Value) -> Vec<ImportedEdge> {
    case["input"]["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .enumerate()
        .map(|(index, edge)| ImportedEdge {
            source: edge[0].as_str().expect("source").to_string(),
            target: edge[1].as_str().expect("target").to_string(),
            family: parse_family(edge[2].as_str().expect("family")),
            origin: format!("stage-edge-{index}"),
        })
        .collect()
}

fn assert_retained(made: &Projection, case: &serde_json::Value) {
    let kept: BTreeSet<String> = made.graph.nodes.iter().map(|n| n.id.clone()).collect();
    let expected: BTreeSet<String> = case["expected"]["retained_nodes"]
        .as_array()
        .expect("retained")
        .iter()
        .map(|v| v.as_str().expect("id").to_string())
        .collect();
    assert_eq!(kept, expected, "{}", case["id"]);
}

fn assert_edges(made: &Projection, case: &serde_json::Value) {
    let mut made_edges: Vec<(String, String, EdgeFamily)> = made
        .graph
        .edges
        .iter()
        .map(|e| (e.source.clone(), e.target.clone(), e.family))
        .collect();
    let mut expected: Vec<(String, String, EdgeFamily)> = case["expected"]["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .map(|e| {
            (
                e[0].as_str().expect("s").to_string(),
                e[1].as_str().expect("t").to_string(),
                parse_family(e[2].as_str().expect("f")),
            )
        })
        .collect();
    made_edges.sort();
    expected.sort();
    assert_eq!(made_edges, expected, "{}", case["id"]);
}

fn json_numbers(value: &serde_json::Value) -> Vec<u64> {
    value
        .as_array()
        .expect("array")
        .iter()
        .map(|v| v.as_u64().expect("u64"))
        .collect()
}

#[test]
fn interface_cases_serialize_exactly() {
    for case in matching()["interface_cases"].as_array().expect("cases") {
        let inputs: Vec<InterfaceElement> = tuples(&case["inputs"]);
        let outputs: Vec<InterfaceElement> = tuples(&case["outputs"]);
        assert_eq!(
            serialize_interface(&inputs),
            case["expected"]["IntroPara"].as_str().expect("intro"),
            "{}",
            case["id"]
        );
        assert_eq!(
            serialize_interface(&outputs),
            case["expected"]["ReturnPara"].as_str().expect("ret"),
            "{}",
            case["id"]
        );
        assert_eq!(
            case["name"].as_str().expect("name"),
            case["expected"]["Name"].as_str().expect("exp name")
        );
    }
}

fn tuples(value: &serde_json::Value) -> Vec<InterfaceElement> {
    value
        .as_array()
        .expect("tuples")
        .iter()
        .map(|tuple| InterfaceElement {
            role: tuple[0].as_str().expect("role").to_string(),
            normalized_type: tuple[1].as_str().expect("type").to_string(),
            variadic: tuple[2].as_bool().expect("variadic"),
        })
        .collect()
}

#[test]
fn jaro_winkler_matches_hand_worked_values_and_symmetry() {
    for case in matching()["jaro_winkler_cases"].as_array().expect("cases") {
        let a = case["a"].as_str().expect("a");
        let b = case["b"].as_str().expect("b");
        let expected = fraction(&case["expected"]);
        assert_eq!(jaro_winkler(a, b).expect("jw"), expected, "{}", case["id"]);
        assert_eq!(
            jaro_winkler(b, a).expect("jw"),
            expected,
            "sym {}",
            case["id"]
        );
    }
}

#[test]
fn directed_diameter_matches_expectations() {
    for case in matching()["diameter_cases"].as_array().expect("cases") {
        let func = compact_graph(&case["graph"]["nodes"], &case["graph"]["edges"]);
        let made = project(&func).expect("stage projection succeeds");
        assert_eq!(
            diameter(&made.graph),
            case["expected"].as_u64().expect("d") as usize,
            "{}",
            case["id"]
        );
    }
}

#[test]
fn pair_depth_matches_expectations() {
    for case in matching()["depth_cases"].as_array().expect("cases") {
        let sizes = json_numbers(&case["sizes"]);
        let diameters = json_numbers(&case["diameters"]);
        assert_eq!(
            pair_depth(
                sizes[0] as usize,
                diameters[0] as usize,
                sizes[1] as usize,
                diameters[1] as usize
            ),
            case["expected"].as_u64().expect("h") as usize,
            "{}",
            case["id"]
        );
    }
}

#[test]
fn wl_stage_matches_expectations() {
    for case in matching()["wl_cases"].as_array().expect("cases") {
        let staged = refine_case(case);
        assert_eq!(
            staged.h,
            staged.expected["h"].as_u64().expect("h") as usize,
            "{}",
            case["id"]
        );
        assert_eq!(
            staged.selected,
            json_numbers(&staged.expected["weights"]),
            "{}",
            case["id"]
        );
        let scores = score_rounds(&staged.rounds, &staged.selected).expect("scores");
        assert_round_vectors(&scores, staged.expected, &case["id"]);
        assert_overlap(&scores, staged.sizes, staged.expected, &case["id"]);
        assert_cosine(&scores, staged.expected, &case["id"]);
    }
}

struct StagedRefinement<'a> {
    h: usize,
    selected: Vec<u64>,
    rounds: Vec<(Histogram, Histogram)>,
    sizes: (u64, u64),
    expected: &'a serde_json::Value,
}

fn refine_case(case: &serde_json::Value) -> StagedRefinement<'_> {
    let func_a = compact_graph(&case["a"]["nodes"], &case["a"]["edges"]);
    let func_b = compact_graph(&case["b"]["nodes"], &case["b"]["edges"]);
    let graph_a = project(&func_a).expect("project a").graph;
    let graph_b = project(&func_b).expect("project b").graph;
    let h = pair_depth(
        graph_a.node_count(),
        diameter(&graph_a),
        graph_b.node_count(),
        diameter(&graph_b),
    );
    let selected = weights(h);
    let refinement = refine(&graph_a, &graph_b, h);
    let rounds: Vec<(Histogram, Histogram)> = refinement
        .rounds
        .iter()
        .map(|(a, b)| (labeled(a), labeled(b)))
        .collect();
    let sizes = (
        graph_a.node_count().min(graph_b.node_count()) as u64,
        graph_a.node_count().max(graph_b.node_count()) as u64,
    );
    StagedRefinement {
        h,
        selected,
        rounds,
        sizes,
        expected: &case["expected"],
    }
}

fn assert_round_vectors(
    scores: &RoundScores,
    expected: &serde_json::Value,
    id: &serde_json::Value,
) {
    assert_eq!(
        scores.overlap_per_round,
        json_numbers(&expected["overlap_per_round"]),
        "{id}"
    );
    assert_eq!(
        scores.cross_per_round,
        json_u128(&expected["cross_per_round"]),
        "{id}"
    );
    assert_eq!(
        scores.self_a_per_round,
        json_u128(&expected["self_a_per_round"]),
        "{id}"
    );
    assert_eq!(
        scores.self_b_per_round,
        json_u128(&expected["self_b_per_round"]),
        "{id}"
    );
}

fn assert_overlap(
    scores: &RoundScores,
    sizes: (u64, u64),
    expected: &serde_json::Value,
    id: &serde_json::Value,
) {
    let overlap = overlap_score(scores, sizes.0, sizes.1).expect("overlap");
    assert_eq!(overlap.score, fraction(&expected["overlap_score"]), "{id}");
    if let Some(coverage) = expected.get("larger_coverage") {
        assert_eq!(overlap.larger_coverage, fraction(coverage), "{id}");
    }
}

fn assert_cosine(scores: &RoundScores, expected: &serde_json::Value, id: &serde_json::Value) {
    let cosine = cosine_score(scores).expect("cosine");
    assert_eq!(
        cosine.kab,
        expected["cosine_score"]["numerator"].as_u64().expect("kab") as u128,
        "{id}"
    );
    assert_eq!(
        cosine.kaa * cosine.kbb,
        expected["cosine_score"]["sqrt_denominator"]
            .as_u64()
            .expect("den") as u128,
        "{id}"
    );
}

fn labeled(hist: &BTreeMap<u64, u64>) -> Histogram {
    hist.iter()
        .map(|(label, count)| (format!("L{label}"), *count))
        .collect()
}

fn json_u128(value: &serde_json::Value) -> Vec<u128> {
    value
        .as_array()
        .expect("array")
        .iter()
        .map(|v| v.as_u64().expect("u64") as u128)
        .collect()
}

#[test]
fn histogram_scoring_matches_expectations() {
    for case in matching()["histogram_score_cases"]
        .as_array()
        .expect("cases")
    {
        let h = case["h"].as_u64().expect("h") as usize;
        let sizes = json_numbers(&case["sizes"]);
        let rounds: Vec<(Histogram, Histogram)> = case["a"]
            .as_array()
            .expect("a")
            .iter()
            .zip(case["b"].as_array().expect("b").iter())
            .map(|(a, b)| (string_hist(a), string_hist(b)))
            .collect();
        assert_eq!(rounds.len(), h + 1, "{}", case["id"]);
        let scores = score_rounds(&rounds, &weights(h)).expect("scores");
        let expected = &case["expected"];
        assert_round_vectors(&scores, expected, &case["id"]);
        let smaller = sizes[0].min(sizes[1]);
        let larger = sizes[0].max(sizes[1]);
        assert_overlap(&scores, (smaller, larger), expected, &case["id"]);
        assert_cosine(&scores, expected, &case["id"]);
        if let Some(raw) = expected.get("forbidden_raw_cross_min_score") {
            assert_raw_cross_unbounded(&scores, smaller, raw, &case["id"]);
        }
    }
}

fn assert_raw_cross_unbounded(
    scores: &RoundScores,
    smaller: u64,
    raw: &serde_json::Value,
    id: &serde_json::Value,
) {
    let cosine = cosine_score(scores).expect("cosine");
    let unbounded = Frac::new(cosine.kab as i128, smaller as i128).expect("frac");
    assert_eq!(unbounded, fraction(raw), "{id}");
    assert!(
        unbounded.numerator() > unbounded.denominator(),
        "raw cross/min is unbounded"
    );
}

fn string_hist(value: &serde_json::Value) -> Histogram {
    value
        .as_object()
        .expect("histogram")
        .iter()
        .map(|(k, v)| (k.clone(), v.as_u64().expect("count")))
        .collect()
}

#[test]
fn inclusive_boundaries_match_expectations() {
    for case in matching()["boundary_cases"].as_array().expect("cases") {
        let threshold =
            Frac::parse_decimal(case["threshold"].as_str().expect("threshold")).expect("threshold");
        let accepted = boundary_accepts(case, threshold);
        assert_eq!(
            accepted,
            case["expected"].as_bool().expect("expected"),
            "{}",
            case["id"]
        );
    }
}

fn boundary_accepts(case: &serde_json::Value, threshold: Frac) -> bool {
    match case["kind"].as_str().expect("kind") {
        "ratio" => {
            let sizes = json_numbers(&case["sizes"]);
            let (smaller, larger) = (sizes[0].min(sizes[1]), sizes[0].max(sizes[1]));
            smaller > 0
                && Frac::new(smaller as i128, larger as i128)
                    .expect("ratio")
                    .ge(threshold)
                    .expect("cmp")
        }
        "rational" => fraction(&case["score"]).ge(threshold).expect("cmp"),
        "mean" => {
            let fields: Vec<Frac> = case["fields"]
                .as_array()
                .expect("fields")
                .iter()
                .map(fraction)
                .collect();
            let sum = fields[0]
                .checked_add(fields[1])
                .expect("add")
                .checked_add(fields[2])
                .expect("add");
            sum.checked_div(Frac::new(3, 1).expect("three"))
                .expect("div")
                .ge(threshold)
                .expect("cmp")
        }
        _ => {
            let dot = case["dot"].as_u64().expect("dot") as u128;
            let norm_product = case["norm_product"].as_u64().expect("norms") as u128;
            cosine_gate(dot, norm_product, threshold)
        }
    }
}

fn cosine_gate(dot: u128, norm_product: u128, threshold: Frac) -> bool {
    let left = dot * dot * (threshold.denominator() * threshold.denominator()) as u128;
    let right = (threshold.numerator() * threshold.numerator()) as u128 * norm_product;
    left >= right
}

#[test]
fn navigation_ids_and_collection_order_do_not_change_scores() {
    let a = compact_graph(
        &serde_json::json!({"a": "DECL", "b": "ASSIGN"}),
        &serde_json::json!([["a", "b", "data"]]),
    );
    let renamed = compact_graph(
        &serde_json::json!({"z": "ASSIGN", "q": "DECL"}),
        &serde_json::json!([["q", "z", "data"]]),
    );
    let graph_a = project(&a).expect("a").graph;
    let graph_z = project(&renamed).expect("z").graph;
    assert_same_scores(&graph_a, &graph_z, 1);
    let forward = compact_graph(
        &serde_json::json!({"a": "DECL", "b": "ASSIGN", "c": "OTHER"}),
        &serde_json::json!([["a", "b", "data"], ["b", "c", "control"]]),
    );
    let backward = compact_graph(
        &serde_json::json!({"a": "DECL", "b": "ASSIGN", "c": "OTHER"}),
        &serde_json::json!([["b", "c", "control"], ["a", "b", "data"]]),
    );
    let graph_f = project(&forward).expect("forward").graph;
    let graph_b = project(&backward).expect("backward").graph;
    assert_same_scores(&graph_f, &graph_b, 2);
}

fn assert_same_scores(a: &ProjectedGraph, b: &ProjectedGraph, h: usize) {
    let rounds = |x: &ProjectedGraph, y: &ProjectedGraph| {
        refine(x, y, h)
            .rounds
            .iter()
            .map(|(p, q)| (labeled(p), labeled(q)))
            .collect::<Vec<_>>()
    };
    let selected = weights(h);
    let first = score_rounds(&rounds(a, b), &selected).expect("scores");
    let swapped = score_rounds(&rounds(b, a), &selected).expect("scores");
    assert_eq!(first.overlap_per_round, swapped.overlap_per_round);
    assert_eq!(first.cross_per_round, swapped.cross_per_round);
    assert_eq!(first.self_a_per_round, swapped.self_b_per_round);
    assert_eq!(first.self_b_per_round, swapped.self_a_per_round);
}
