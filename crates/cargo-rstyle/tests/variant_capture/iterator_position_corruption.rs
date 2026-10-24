use super::*;
use rstyle_pdg::producer::Pair;

#[test]
fn byte_iterator_independently_rejects_forged_prefix_order_result_and_authority() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "pub fn search(bytes: &[u8], byte: u8) -> Option<usize> { bytes.iter().position(|&b| b == byte) }").unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    let function = &report["functions"][0];
    assert_eq!(function["outcome"], "admitted");
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = rstyle_pdg::producer::produce(&facts).unwrap();
    rstyle_ccgraph::admit_produced(&original, &facts).unwrap();

    check_document_mutations(&facts);
    for mutation in [
        "implementation",
        "iterator",
        "environment",
        "pattern",
        "equality",
        "receiver",
    ] {
        let mut altered = (*facts).clone();
        corrupt_facts(&mut altered, mutation);
        shared_evidence::reject_facts(function, &altered);
        assert!(
            rstyle_pdg::data_documents::verify(&original, &altered).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_pdg::producer::produce(&altered).is_err(),
            "{mutation}"
        );
    }
}

fn check_document_mutations(facts: &dyn rstyle_pdg::unit_store::EvidenceFacts) {
    for mutation in [
        "reads",
        "semantics",
        "type",
        "effects",
        "operand",
        "true",
        "false",
        "result",
        "inventory",
        "policy",
    ] {
        let mut pair = rstyle_pdg::producer::produce(facts).unwrap();
        corrupt(&mut pair, mutation);
        assert!(
            rstyle_pdg::data_documents::verify(&pair, facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&pair, facts).is_err(),
            "{mutation}"
        );
    }
}

fn corrupt(pair: &mut Pair, mutation: &str) {
    let prefix = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("iterator.byte-position-prefix"))
        .unwrap();
    match mutation {
        "reads" => prefix.attributes.reads.clear(),
        "semantics" => {
            prefix
                .annotations
                .get_mut("rstyle.data:byte-position")
                .unwrap()["read_prefix"] = "entire-view".into()
        }
        "type" => {
            prefix
                .annotations
                .get_mut("compat.rust.hir-r0:type")
                .unwrap()["value"] = "usize".into()
        }
        "effects" => prefix.attributes.writes = prefix.attributes.reads.clone(),
        "operand" => pair
            .pdg
            .edges
            .retain(|edge| !(edge.target == prefix.id && edge.subkind == "value_operand")),
        "inventory" => prefix.attributes.construct = Some("unregistered".into()),
        "policy" => {
            pair.pdg.profile.version = "3.53.0".into();
            pair.cfg.profile.version = "3.53.0".into();
        }
        "true" | "false" => bypass_guard(pair, mutation),
        "result" => {
            let sites = pair
                .pdg
                .extensions
                .get_mut("compat.rust.hir-r0:value-sites")
                .unwrap()
                .as_array_mut()
                .unwrap();
            let alternatives = sites[0]["alternatives"].as_array_mut().unwrap();
            alternatives[0]["outcome"] = "false".into();
        }
        _ => unreachable!(),
    }
}

fn bypass_guard(pair: &mut Pair, outcome: &str) {
    let selector = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some("iterator.nonempty"))
        .unwrap();
    let map = rstyle_pdg::read_operation_map(&pair.pdg.extensions).unwrap();
    let cfg = &map
        .iter()
        .find(|entry| entry.pdg_node_id == selector.id)
        .unwrap()
        .cfg_node_id;
    let kind = if outcome == "true" {
        rstyle_pdg::CfgEdgeKind::True
    } else {
        rstyle_pdg::CfgEdgeKind::False
    };
    let edge = pair
        .cfg
        .edges
        .iter_mut()
        .find(|edge| edge.source == *cfg && edge.kind == kind)
        .unwrap();
    edge.kind = rstyle_pdg::CfgEdgeKind::Normal;
}

fn corrupt_facts(facts: &mut rstyle_pdg::HirFnFacts, mutation: &str) {
    if mutation == "receiver" {
        facts
            .build
            .frontend_options
            .get_mut("data_method_receivers")
            .unwrap()["methods"][0]["signature_inputs"] = serde_json::json!([]);
        return;
    }
    let position = find_mut(&mut facts.body, "expr_method_call").unwrap();
    match mutation {
        "implementation" | "iterator" => {
            let node = if mutation == "iterator" {
                &mut position.children[0]
            } else {
                position
            };
            let resolution = node.res.as_mut().unwrap();
            let mut callee: Value = serde_json::from_str(&resolution.description).unwrap();
            if mutation == "iterator" {
                callee["definition_hash"] = "forged".into();
            } else {
                callee["implementation"]["definition_hash"] = "forged".into();
            }
            resolution.description = callee.to_string();
        }
        "environment" => {
            let resolution = position.children[1].res.as_mut().unwrap();
            let mut environment: Value = serde_json::from_str(&resolution.description).unwrap();
            environment["captures"] = serde_json::json!([]);
            resolution.description = environment.to_string();
        }
        "pattern" => position.children[1].children[0].children[0].binding_mut = Some(true),
        "equality" => {
            position.children[1].children[1]
                .dispatch
                .as_mut()
                .unwrap()
                .op = "!=".into()
        }
        _ => unreachable!(),
    }
}

fn find_mut<'a>(
    node: &'a mut rstyle_pdg::HirNode,
    kind: &str,
) -> Option<&'a mut rstyle_pdg::HirNode> {
    if node.kind == kind {
        return Some(node);
    }
    node.children
        .iter_mut()
        .find_map(|child| find_mut(child, kind))
}
