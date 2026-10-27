//! Complete checked reads, saved origins and helper composition through strict import.
use super::*;

#[test]
fn checked_reads_compose_through_strict_documents_and_matching() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 7);
    for function in functions {
        assert_eq!(
            function["outcome"],
            if function["facts"]["name"] == "mutable" {
                "excluded"
            } else {
                "admitted"
            },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    for name in ["saved", "snapshot", "helper"] {
        let function = functions
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        retained_first_origin(function);
    }
    check_equivalent_matching(&report, ["boundary", "same"]);
    let boundary = functions
        .iter()
        .find(|f| f["facts"]["name"] == "boundary")
        .unwrap();
    corruptions(boundary);
}

fn source() -> &'static str {
    r#"
    pub fn boundary(bytes: &[u8], i: usize) -> bool {
        match bytes.get(i) { None => i == bytes.len(), Some(&b) => b <= 127 || b >= 192 }
    }
    pub fn same(bytes: &[u8], i: usize) -> bool {
        match bytes.get(i) { None => i == bytes.len(), Some(&b) => b <= 127 || b >= 192 }
    }
    pub fn checked(bytes: &[u8], i: usize) -> u8 {
        match bytes.get(i) { Some(&b) => b, None => 0 }
    }
    pub fn saved(a: &[u8], b: &[u8], i: usize) -> u8 {
        let mut selected = a; let value = selected.get(i); selected = b;
        match value { Some(&byte) => byte, None => 0 }
    }
    pub fn snapshot(a: &[u8], b: &[u8]) -> u8 {
        let mut selected = a;
        match selected.get({ selected = b; 0 }) { Some(&byte) => byte, None => 0 }
    }
    pub fn helper(a: &[u8], i: usize) -> bool { boundary(a, i) }
    pub fn mutable(a: &mut [u8], i: usize) -> u8 {
        match a.get_mut(i) { Some(byte) => *byte, None => 0 }
    }
    "#
}

fn corruptions(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for change in [
        "get-rule",
        "get-read",
        "get-operand",
        "variant",
        "payload-read",
        "payload-binding",
        "payload-operand",
        "guard",
    ] {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        corrupt_pair(&mut pair, change);
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "{change}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
            "{change}"
        );
    }
    for construct in [
        "data.option-get",
        "data.option-payload",
        "match.option-some",
        "match.option-none",
    ] {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        let node = pair
            .pdg
            .nodes
            .iter_mut()
            .find(|node| node.attributes.construct.as_deref() == Some(construct))
            .unwrap();
        node.annotations.remove("rstyle.data:checked-option");
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "{construct}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
            "{construct}"
        );
    }
}

fn corrupt_pair(pair: &mut rstyle_pdg::producer::Pair, change: &str) {
    let construct = match change {
        "get-rule" | "get-read" | "get-operand" => "data.option-get",
        "variant" | "guard" => "match.option-some",
        _ => "data.option-payload",
    };
    let position = pair
        .pdg
        .nodes
        .iter()
        .position(|node| node.attributes.construct.as_deref() == Some(construct))
        .unwrap();
    let id = pair.pdg.nodes[position].id.clone();
    match change {
        "get-rule" => {
            pair.pdg.nodes[position]
                .annotations
                .get_mut("rstyle.data:checked-option")
                .unwrap()["failure"] = "some".into()
        }
        "get-read" => invent_get_read(pair, position),
        "variant" => {
            pair.pdg.nodes[position].attributes.construct = Some("match.option-none".into())
        }
        "payload-read" => pair.pdg.nodes[position].attributes.reads.clear(),
        "payload-binding" => {
            let declaration = pair
                .pdg
                .nodes
                .iter_mut()
                .find(|node| node.category == rstyle_pdg::NodeCategory::Declaration)
                .unwrap();
            declaration.attributes.symbol_ids.clear();
        }
        "get-operand" | "payload-operand" => {
            let edge = pair
                .pdg
                .edges
                .iter_mut()
                .find(|edge| {
                    edge.target == id
                        && edge.subkind == "value_operand"
                        && edge.operand_position == Some(0)
                })
                .unwrap();
            edge.source = id;
        }
        "guard" => corrupt_guard(pair, &id),
        _ => unreachable!(),
    }
}

fn retained_first_origin(function: &Value) {
    let (unit, _) = data_unit(function);
    let pair = shared_evidence::pair(&unit, function);
    assert_eq!(pair.pdg.memory_locations.len(), 1);
    let location = &pair.pdg.memory_locations[0];
    assert_eq!(
        location.extensions[rstyle_pdg::data_documents::LOCATION_KEY]["parameter"],
        0
    );
}

fn corrupt_guard(pair: &mut rstyle_pdg::producer::Pair, id: &str) {
    let map = rstyle_pdg::read_operation_map(&pair.pdg.extensions).unwrap();
    let cfg = &map
        .iter()
        .find(|entry| entry.pdg_node_id == id)
        .unwrap()
        .cfg_node_id;
    let edge = pair
        .cfg
        .edges
        .iter_mut()
        .find(|edge| edge.source == *cfg && edge.kind == rstyle_pdg::CfgEdgeKind::True)
        .unwrap();
    edge.kind = rstyle_pdg::CfgEdgeKind::False;
}

fn invent_get_read(pair: &mut rstyle_pdg::producer::Pair, position: usize) {
    let reads = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some("data.option-payload"))
        .unwrap()
        .attributes
        .reads
        .clone();
    pair.pdg.nodes[position].attributes.reads = reads;
}
