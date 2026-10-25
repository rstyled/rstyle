use super::*;
use rstyle_pdg::unit_store::EvidenceFacts;

pub(super) fn reject_source_ownership(function: &Value) {
    for attack in ["implementation", "declaration"] {
        shared_evidence::reject_edit(function, |bundle, index| {
            let record =
                &mut bundle["functions"][index]["build"]["frontend_options"]["inherent_definition"];
            let identity = record["implementation_identity"].clone();
            if attack == "implementation" {
                let start = record["implementation_span"]["start"].as_u64().unwrap();
                record["implementation_span"]["start"] = (start + 1).into();
            } else {
                record["owner_nominal"]["declaration_digest"] = "xxh3-64:0000000000000000".into();
            }
            let implementation = record["implementation_span"].clone();
            let owner = record["owner_nominal"].clone();
            let mut changed = 0;
            for inventory in bundle["inherent_units"].as_array_mut().unwrap() {
                for association in inventory["definitions"].as_array_mut().unwrap() {
                    if association["implementation_identity"] == identity {
                        association["implementation_span"] = implementation.clone();
                        association["owner_nominal"] = owner.clone();
                        changed += 1;
                    }
                }
            }
            assert_eq!(changed, 1, "edit both coherent ownership records");
        });
    }
}

pub(super) fn reject_all(original: &Pair, facts: &dyn EvidenceFacts) {
    for attack in [
        "selector-location",
        "selector-omission",
        "body-load-omission",
        "selector-reuse",
        "guard-bypass",
        "suffix-start",
        "some-field",
        "flattened-origin",
        "none-payload",
        "constructor-swap",
        "old-policy",
    ] {
        let mut pair = Pair {
            pdg: original.pdg.clone(),
            cfg: original.cfg.clone(),
        };
        edit(&mut pair, attack);
        assert!(
            rstyle_pdg::data_documents::verify(&pair, facts).is_err(),
            "independent document verifier accepted {attack}"
        );
        assert!(
            rstyle_pdg::producer::verify(&pair, facts).is_err(),
            "producer replay accepted {attack}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&pair, facts).is_err(),
            "strict admission accepted {attack}"
        );
    }
}

fn edit(pair: &mut Pair, attack: &str) {
    match attack {
        "selector-location" | "selector-omission" | "body-load-omission" | "selector-reuse" => {
            reads(pair, attack)
        }
        "guard-bypass" => {
            guard(pair);
        }
        "suffix-start" => {
            node_mut(pair, "slice.prefix-suffix").attributes.literal = Some(
                rstyle_pdg::StringFact::known("{\"kind\":\"suffix\",\"start\":0}"),
            );
        }
        "some-field" | "none-payload" => components(pair, attack),
        "flattened-origin" => {
            node_mut(pair, "data.digit-some").annotations.insert(
                rstyle_pdg::data_origins::KEY.into(),
                serde_json::json!([{"parameter":0,"path":[]}]),
            );
        }
        "constructor-swap" => {
            let product = node(pair, "data.scalar-product").id.clone();
            for edge in pair
                .pdg
                .edges
                .iter_mut()
                .filter(|edge| edge.target == product && edge.subkind == "value_operand")
            {
                edge.operand_position = edge.operand_position.map(|position| 1 - position);
            }
        }
        "old-policy" => {
            pair.pdg.profile.version = "3.108.0".into();
            pair.cfg.profile.version = "3.108.0".into();
        }
        _ => unreachable!(),
    }
}

fn reads(pair: &mut Pair, attack: &str) {
    let construct = if attack == "body-load-omission" || attack == "selector-reuse" {
        "data.indirect-deref"
    } else {
        "slice.prefix-byte"
    };
    if attack == "selector-reuse" {
        let selector = node(pair, "slice.prefix-byte").clone();
        let body = node_mut(pair, construct);
        body.source_span = selector.source_span;
        body.attributes.construct = selector.attributes.construct;
    } else if attack == "selector-location" {
        let different = pair
            .pdg
            .nodes
            .iter()
            .filter(|node| node.attributes.construct.as_deref() == Some("slice.prefix-byte"))
            .nth(1)
            .unwrap()
            .attributes
            .reads[0]
            .location_id
            .clone();
        assert_ne!(
            node(pair, construct).attributes.reads[0].location_id,
            different
        );
        node_mut(pair, construct).attributes.reads[0].location_id = different;
    } else {
        node_mut(pair, construct).attributes.reads.clear();
    }
}

fn node_mut<'a>(pair: &'a mut Pair, construct: &str) -> &'a mut PdgNode {
    pair.pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some(construct))
        .unwrap()
}

fn guard(pair: &mut Pair) {
    let selector = node(pair, "slice.prefix-digit").id.clone();
    let mapping = rstyle_pdg::read_operation_map(&pair.pdg.extensions).unwrap();
    let cfg = &mapping
        .iter()
        .find(|entry| entry.pdg_node_id == selector)
        .unwrap()
        .cfg_node_id;
    let edge = pair
        .cfg
        .edges
        .iter_mut()
        .find(|edge| edge.source == *cfg && edge.kind == rstyle_pdg::CfgEdgeKind::True)
        .unwrap();
    edge.kind = rstyle_pdg::CfgEdgeKind::Normal;
}

fn components(pair: &mut Pair, attack: &str) {
    let none = node(pair, "data.digit-none").id.clone();
    let some = node(pair, "data.digit-some").id.clone();
    let payload =
        origin_value(pair, "data.digit-some")["footprint"]["cases"][0]["components"].clone();
    let values = &mut pair.pdg.extensions.get_mut(ORIGINS).unwrap()["values"];
    let selected = if attack == "some-field" { some } else { none };
    let value = &mut values
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["pdg_node_id"].as_str() == Some(selected.as_str()))
        .unwrap()["value"];
    if attack == "some-field" {
        value["footprint"]["cases"][0]["components"][0]["selection"] = serde_json::json!([0, 1]);
    } else {
        value["footprint"]["cases"][0]["components"] = payload;
    }
}
