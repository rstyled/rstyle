use super::*;

fn selected(report: &Value) -> &Value {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "read_u16")
        .unwrap()
}

#[test]
fn prefix_size_and_native_documents_reject_altered_semantics_and_policy() {
    let (_directory, report) = captured(false, None);
    let function = selected(&report);
    assert_eq!(function["outcome"], "admitted");
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for mutation in [
        "bounds",
        "operand",
        "size",
        "type",
        "literal",
        "significance",
        "copy",
        "location-type",
        "policy",
    ] {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        edit(&mut pair, mutation);
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_pdg::producer::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
            "{mutation}"
        );
    }
}

fn edit(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    if mutation == "location-type" {
        pair.pdg.memory_locations[0]
            .extensions
            .get_mut(rstyle_pdg::data_documents::LOCATION_KEY)
            .unwrap()["type"]["value"]["length"] = 4.into();
        return;
    }
    if mutation == "policy" {
        pair.pdg.profile.version = "3.43.0".into();
        pair.cfg.profile.version = "3.43.0".into();
        return;
    }
    let construct = match mutation {
        "bounds" | "operand" => "data.subslice-place",
        "significance" => "data.bytes.native-u16",
        "copy" => "data.bytes.copy-result",
        _ => "data.scalar-size",
    };
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some(construct))
        .unwrap();
    if mutation == "operand" {
        pair.pdg
            .edges
            .retain(|edge| !(edge.target == node.id && edge.subkind == "value_operand"));
    } else {
        edit_node(node, mutation);
    }
}

fn edit_node(node: &mut rstyle_pdg::PdgNode, mutation: &str) {
    match mutation {
        "bounds" => {
            node.annotations
                .get_mut("rstyle.data:subslice-bounds")
                .unwrap()["implicit_start"] = 1.into();
        }
        "size" => {
            node.annotations.get_mut("rstyle.data:scalar-size").unwrap()["value"] = 4.into();
        }
        "type" => {
            node.annotations.get_mut("compat.rust.hir-r0:type").unwrap()["value"] = "u16".into();
        }
        "literal" => {
            node.attributes.literal = Some(rstyle_pdg::StringFact::known("2"));
        }
        "significance" => {
            node.annotations.get_mut("rstyle.data:byte-call").unwrap()["byte_significance_bits"]
                [0] = 8.into();
        }
        _ => {
            node.annotations.get_mut("rstyle.data:byte-call").unwrap()["length"] = 4.into();
        }
    }
}

#[test]
fn scalar_size_rejects_missing_wrong_or_unpinned_instantiation_evidence() {
    let (_directory, report) = captured(false, None);
    let function = selected(&report);
    for mutation in ["missing", "argument", "core", "definition", "range"] {
        shared_evidence::reject_edit(function, |bundle, index| {
            let body = &mut bundle["functions"][index]["body"];
            edit_resolution(body, mutation);
        });
    }
}

fn edit_resolution(node: &mut Value, mutation: &str) {
    let range = mutation == "range";
    if node["res"]["kind"]
        == if range {
            "resolved_range_type"
        } else {
            "resolved_scalar_size_call"
        }
    {
        let mut resolution: Value =
            serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
        match mutation {
            "missing" => {
                resolution.as_object_mut().unwrap().remove("argument");
            }
            "argument" => {
                resolution["argument"] = "u32".into();
            }
            "core" => {
                resolution["callee"]["crate_hash"] = "wrong".into();
            }
            "definition" => {
                resolution["callee"]["definition_hash"] = "wrong".into();
            }
            _ => {
                resolution["definition_hash"] = "wrong".into();
            }
        }
        node["res"]["description"] = resolution.to_string().into();
    } else if let Some(children) = node["children"].as_array_mut() {
        for child in children {
            edit_resolution(child, mutation);
        }
    }
}
