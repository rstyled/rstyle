//! Mechanism coverage for the retained Input-chain foundation, not a natural-family claim.
use super::*;

const SOURCE: &str = r#"
    pub struct Views<'a> { pub first: &'a [u8], pub second: &'a [u8] }
    pub fn pair<'a>(a: &'a [u8], b: &'a [u8]) -> (&'a [u8], &'a [u8]) { (a, b) }
    pub fn first<'a>(a: &'a [u8], b: &'a [u8]) -> &'a [u8] { pair(a, b).0 }
    pub fn second<'a>(a: &'a [u8], b: &'a [u8]) -> &'a [u8] { pair(a, b).1 }
    pub fn forward<'a>(a: &'a [u8], b: &'a [u8]) -> (&'a [u8], &'a [u8]) { pair(a, b) }
    pub fn saved<'a>(a: &'a [u8], b: &'a [u8]) -> &'a [u8] {
        let mut selected = a; let result = pair(selected, b); selected = b; result.0
    }
    pub fn ordered<'a>(a: &'a [u8], b: &'a [u8]) -> &'a [u8] {
        let mut selected = a; pair(selected, { selected = b; selected }).0
    }
    pub fn nested<'a>(a: &'a [u8], b: &'a [u8]) -> ((&'a [u8], &'a [u8]), &'a [u8]) { (pair(a, b), a) }
    pub fn nested_second<'a>(a: &'a [u8], b: &'a [u8]) -> &'a [u8] { nested(a, b).0.1 }
    pub fn views<'a>(a: &'a [u8], b: &'a [u8]) -> Views<'a> { Views { first: a, second: b } }
    pub fn view_second<'a>(a: &'a [u8], b: &'a [u8]) -> &'a [u8] { views(a, b).second }
    pub fn read_second(a: &[u8], b: &[u8]) -> u8 { pair(a, b).1[0] }
"#;

fn named<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == name)
        .unwrap()
}

#[test]
fn component_helper_results_compose_selected_returns_saved_actuals_and_reads() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "admitted", "{function}");
    }
    check_complete_data_pairs(&report, 11);
    check_aggregate_documents(&report);
    expected_roots(&report);
    corrupt_summaries(named(&report, "pair"));
    corrupt_graph(named(&report, "pair"));
}

fn expected_roots(report: &Value) {
    for (name, parameter) in [
        ("first", 0),
        ("second", 1),
        ("saved", 0),
        ("ordered", 0),
        ("nested_second", 1),
        ("view_second", 1),
    ] {
        let function = named(report, name);
        let (store, _) = data_unit(function);
        let pair = shared_evidence::pair(&store, function);
        let returned = pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
            .unwrap();
        let origins: Vec<rstyle_pdg::data_origins::Origin> =
            serde_json::from_value(returned.annotations[rstyle_pdg::data_origins::KEY].clone())
                .unwrap();
        assert_eq!(origins.len(), 1, "{name}");
        assert_eq!(origins[0].parameter, parameter, "{name}");
    }
}

fn corrupt_summaries(function: &Value) {
    for attack in ["omit", "duplicate", "selector", "splice"] {
        shared_evidence::reject_edit(function, |bundle, index| {
            let helper = bundle["helpers"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|helper| helper["function"] == index)
                .unwrap();
            let leaves = helper["summary"]["component_outputs"]
                .as_array_mut()
                .unwrap();
            match attack {
                "omit" => {
                    leaves.pop();
                }
                "duplicate" => {
                    leaves[1] = leaves[0].clone();
                }
                "selector" => {
                    leaves[1]["selectors"] = serde_json::json!([0]);
                }
                "splice" => {
                    let first = leaves[0]["origins"].clone();
                    leaves[0]["origins"] = leaves[1]["origins"].clone();
                    leaves[1]["origins"] = first;
                }
                _ => unreachable!(),
            }
        });
    }
}

fn corrupt_graph(function: &Value) {
    let (store, index) = data_unit(function);
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = shared_evidence::pair(&store, function);
    let mut pdg = pair.pdg.clone();
    let constructor = pdg
        .nodes
        .iter()
        .find(|node| {
            node.category == rstyle_pdg::NodeCategory::Computation
                && node.original_kind.known_value() == Some("expr_tup")
        })
        .unwrap()
        .id
        .clone();
    for edge in &mut pdg.edges {
        if edge.target == constructor && edge.subkind == "value_operand" {
            edge.operand_position = edge.operand_position.map(|position| 1 - position);
        }
    }
    let changed = rstyle_pdg::producer::PairView {
        pdg: &pdg,
        cfg: &pair.cfg,
    };
    assert!(rstyle_pdg::helper_component_outputs::derive(&facts, changed).is_err());
}
