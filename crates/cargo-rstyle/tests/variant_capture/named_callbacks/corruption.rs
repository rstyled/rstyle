use super::*;

#[test]
fn named_callback_documents_reject_identity_payload_order_effect_and_result_forgeries() {
    let (_directory, report) = captured();
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "eager")
        .unwrap();
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for mutation in [
        "missing",
        "payload",
        "argument",
        "callee",
        "effect",
        "type",
        "result",
        "early-item",
        "none-call",
        "none-payload",
        "policy",
    ] {
        let captured = unit.pair(index).unwrap();
        let mut pair = rstyle_pdg::producer::Pair {
            pdg: captured.pdg.clone(),
            cfg: captured.cfg.clone(),
        };
        corrupt(&mut pair, mutation);
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
    named_callbacks_require_resolved_function_items_and_verified_summaries(&report);
}

fn named_callbacks_require_resolved_function_items_and_verified_summaries(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "eager")
        .unwrap();
    for mutation in [
        "identity",
        "signature",
        "coercion",
        "source",
        "missing-resolution",
        "missing-summary",
        "invented-purity",
    ] {
        shared_evidence::reject_edit(function, |bundle, index| {
            edit_facts(bundle, index, mutation)
        });
    }
}

fn node(pair: &rstyle_pdg::producer::Pair, construct: &str) -> String {
    pair.pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some(construct))
        .unwrap()
        .id
        .clone()
}

fn corrupt(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    let payload = node(pair, "callback.named-option-payload");
    let call = node(pair, "callback.named-invoke");
    match mutation {
        "payload" | "argument" => pair.pdg.edges.retain(|edge| {
            !(edge.target
                == *if mutation == "payload" {
                    &payload
                } else {
                    &call
                }
                && edge.subkind == "value_operand")
        }),
        "missing" => pair.pdg.nodes.retain(|node| node.id != payload),
        "result" => corrupt_result(pair),
        "policy" => {
            pair.pdg.profile.version = "3.56.0".into();
            pair.cfg.profile.version = "3.56.0".into();
        }
        "early-item" | "none-call" | "none-payload" => bypass(pair, mutation),
        _ => {
            let call = pair
                .pdg
                .nodes
                .iter_mut()
                .find(|node| node.id == call)
                .unwrap();
            match mutation {
                "callee" => {
                    call.attributes.callee = Some(rstyle_pdg::StringFact::known("invented"))
                }
                "effect" => {
                    call.attributes.literal = Some(rstyle_pdg::StringFact::known("invented"))
                }
                "type" => {
                    call.annotations.get_mut("compat.rust.hir-r0:type").unwrap()["value"] =
                        "u8".into()
                }
                _ => unreachable!(),
            }
        }
    }
}

fn cfg(pair: &rstyle_pdg::producer::Pair, id: &str) -> String {
    rstyle_pdg::read_operation_map(&pair.pdg.extensions)
        .unwrap()
        .into_iter()
        .find(|entry| entry.pdg_node_id == id)
        .unwrap()
        .cfg_node_id
}

fn bypass(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    let selector = cfg(pair, &node(pair, "callback.named-option-some"));
    let target = match mutation {
        "early-item" => "callback.function-item",
        "none-call" => "callback.named-invoke",
        "none-payload" => "callback.named-option-payload",
        _ => unreachable!(),
    };
    let mut edge = pair
        .cfg
        .edges
        .iter()
        .find(|edge| edge.source == selector)
        .unwrap()
        .clone();
    edge.source = if mutation == "early-item" {
        pair.cfg.entry_node_id.clone()
    } else {
        selector
    };
    edge.target = cfg(pair, &node(pair, target));
    edge.kind = rstyle_pdg::CfgEdgeKind::Normal;
    pair.cfg.edges.push(edge);
}

fn corrupt_result(pair: &mut rstyle_pdg::producer::Pair) {
    for extensions in [&mut pair.pdg.extensions, &mut pair.cfg.extensions] {
        let alternatives = extensions
            .get_mut("compat.rust.hir-r0:value-sites")
            .unwrap()[0]["alternatives"]
            .as_array_mut()
            .unwrap();
        let some = alternatives
            .iter()
            .find(|alt| alt["outcome"] == "true")
            .unwrap()["value_pdg_node_id"]
            .clone();
        alternatives
            .iter_mut()
            .find(|alt| alt["outcome"] == "false")
            .unwrap()["value_pdg_node_id"] = some;
    }
}

fn edit_facts(bundle: &mut Value, index: usize, mutation: &str) {
    if mutation == "missing-summary" || mutation == "invented-purity" {
        let helper = bundle["helpers"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|helper| helper["summary"]["argument_types"] == serde_json::json!(["u8"]))
            .unwrap();
        if mutation == "missing-summary" {
            helper["summary"] = Value::Null;
        } else {
            helper["summary"]["may_panic"] = true.into();
        }
        return;
    }
    assert!(edit_item(&mut bundle["functions"][index]["body"], mutation));
}

fn edit_item(node: &mut Value, mutation: &str) -> bool {
    if node["res"]["kind"] == "resolved_function_item" {
        match mutation {
            "missing-resolution" => node["res"] = Value::Null,
            "signature" => node["dispatch"]["operand_tys"] = serde_json::json!(["u16"]),
            "coercion" => node["ty"]["normalized"] = "bool".into(),
            "source" => node["span"]["start"] = 0.into(),
            "identity" => {
                let mut resolution: Value =
                    serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
                resolution["definition_identity"] = "other".into();
                node["res"]["description"] = resolution.to_string().into();
            }
            _ => unreachable!(),
        }
        return true;
    }
    node["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .any(|child| edit_item(child, mutation))
}
