//! Exact backing and saved-width expectations for original new_wrapped.
use super::*;
use rstyle_pdg::{NodeCategory, PdgNode, unit_store::EvidenceFacts};

const UPDATE: &str = "data.owned-product-update";
const GUARD: &str = "try.product-result-is-err";
const PAYLOAD: &str = "try.product-result-ok-product";
const ERROR: &str = "try.product-result-error";

pub(super) fn check(pair: &Pair, view: &dyn EvidenceFacts) -> Result<usize, String> {
    let expected = json!([{"parameter":0,"path":[{"kind":"SharedReferent"}]}]);
    for (construct, roots) in [
        ("data.result-ok", &expected),
        (PAYLOAD, &expected),
        (UPDATE, &expected),
        ("data.result-err", &json!([])),
        (ERROR, &json!([])),
    ] {
        if unique(pair, construct)?
            .annotations
            .get(rstyle_pdg::data_origins::KEY)
            != Some(roots)
        {
            return Err(format!(
                "Original wrapped {construct} differs from backing oracle"
            ));
        }
    }
    check_width(pair, view)?;
    check_saved(pair, view)?;
    check_helper(view, &expected)?;
    verify_attacks(
        pair,
        view,
        &[
            "stale-input",
            "swapped-input",
            "update-rhs",
            "wrong-field",
            "wrong-owner",
            "missing-update",
            "duplicate-update",
            "fake-place",
            "fake-memory",
            "product-guard",
            "product-payload",
            "product-error",
            "product-control",
            "saved-result",
            "ok-root",
            "err-root",
            "update-root",
            "policy",
        ],
        forge,
    )
}

fn check_saved(pair: &Pair, view: &dyn EvidenceFacts) -> Result<(), String> {
    let calls: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.category == NodeCategory::Call
                && node
                    .attributes
                    .callee
                    .as_ref()
                    .and_then(rstyle_pdg::StringFact::known_value)
                    .is_some_and(|name| name.ends_with("::new_unwrapped"))
        })
        .collect();
    let [call] = calls.as_slice() else {
        return Err("Expected one original initializer helper".into());
    };
    let bytes = operand(pair, call, 0)?;
    if bytes.category != NodeCategory::Read
        || bytes.attributes.symbol_ids != vec![view.facts().signature.params[0].symbol.clone()]
    {
        return Err("Initializer must consume the saved bytes formal".into());
    }
    for construct in [GUARD, PAYLOAD, ERROR] {
        if operand(pair, unique(pair, construct)?, 0)?.id != call.id {
            return Err(
                "Product tag and disjoint transfers must share the complete saved result".into(),
            );
        }
    }
    let update = unique(pair, UPDATE)?;
    let old = operand(pair, update, 0)?;
    let initial = definition(pair, old)?;
    let returned = operand(pair, unique(pair, "data.result-ok")?, 0)?;
    let assigned = definition(pair, returned)?;
    if initial.category != NodeCategory::Declaration
        || assigned.category != NodeCategory::Assignment
        || operand(pair, initial, 0)?.id != unique(pair, PAYLOAD)?.id
        || operand(pair, assigned, 0)?.id != update.id
    {
        return Err("Wrapped success must carry the initialized then updated whole owner".into());
    }
    Ok(())
}

fn definition<'a>(pair: &'a Pair, read: &PdgNode) -> Result<&'a PdgNode, String> {
    let mut edges = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| edge.target == read.id && edge.subkind == "reaching_definition");
    let edge = edges.next().ok_or("Missing exact saved owner definition")?;
    if edges.next().is_some() {
        return Err("Ambiguous owner definition".into());
    }
    pair.pdg
        .nodes
        .iter()
        .find(|node| node.id == edge.source)
        .ok_or("Missing owner definition".into())
}

fn unique<'a>(pair: &'a Pair, construct: &str) -> Result<&'a PdgNode, String> {
    let mut matches = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some(construct));
    let node = matches
        .next()
        .ok_or_else(|| format!("Missing {construct}"))?;
    if matches.next().is_some() {
        return Err(format!("Duplicate {construct}"));
    }
    Ok(node)
}

fn operand<'a>(pair: &'a Pair, node: &PdgNode, position: u64) -> Result<&'a PdgNode, String> {
    let mut edges = pair.pdg.edges.iter().filter(|edge| {
        edge.target == node.id
            && edge.subkind == "value_operand"
            && edge.operand_position == Some(position)
    });
    let edge = edges.next().ok_or("Missing saved operand")?;
    if edges.next().is_some() {
        return Err("Duplicate saved operand".into());
    }
    pair.pdg
        .nodes
        .iter()
        .find(|node| node.id == edge.source)
        .ok_or("Missing operand node".into())
}

fn check_width(pair: &Pair, view: &dyn EvidenceFacts) -> Result<(), String> {
    let update = unique(pair, UPDATE)?;
    let some = unique(pair, "data.option-some")?;
    let width = operand(pair, some, 0)?;
    let parameter = &view.facts().signature.params[1];
    if update
        .attributes
        .literal
        .as_ref()
        .and_then(rstyle_pdg::StringFact::known_value)
        != Some("1")
        || operand(pair, update, 1)?.id != some.id
        || width.category != NodeCategory::Read
        || width.attributes.symbol_ids != vec![parameter.symbol.clone()]
        || unique(pair, "data.result-ok")?.category != NodeCategory::Computation
        || !pair.pdg.memory_locations.is_empty()
    {
        return Err(
            "Wrapped result must save width Some(param1) in field1 without memory effects".into(),
        );
    }
    Ok(())
}

fn check_helper(view: &dyn EvidenceFacts, expected: &Value) -> Result<(), String> {
    let mut calls = Vec::new();
    helper_occurrences(&view.facts().body, &mut calls);
    let [call] = calls.as_slice() else {
        return Err("Expected exactly one unchanged new_unwrapped helper call".into());
    };
    let helper = view
        .helper(call)?
        .ok_or("Missing retained helper dependency")?;
    let summary = helper.summary();
    let outputs = summary
        .component_outputs
        .as_ref()
        .ok_or("Missing helper product outputs")?;
    if summary.identity != "rstyle.helper.guarded-product-result/1"
        || summary.result_dependence != "guarded-saved-result-with-verified-callee-return-graph/1"
        || summary.argument_evaluation != "source-order-once/1"
        || summary.result_type.as_str()
            != call
                .ty
                .as_ref()
                .and_then(|ty| ty.normalized.as_deref())
                .unwrap_or("")
        || summary.may_panic
        || !summary.always_returns
        || summary.memory_effects
        || summary.mutates_arguments
        || summary.bounded_loop.is_some()
        || summary.reference_outputs.is_some()
        || summary.memory_reads.is_some()
        || outputs.len() != 1
        || outputs[0].selectors != vec![0]
        || serde_json::to_value(&outputs[0].origins).map_err(|e| e.to_string())? != *expected
    {
        return Err(
            "Wrapped initializer loses complete guarded saved-callee-result contract".into(),
        );
    }
    Ok(())
}

fn helper_occurrences<'a>(node: &'a rstyle_pdg::HirNode, calls: &mut Vec<&'a rstyle_pdg::HirNode>) {
    if node.kind == "expr_call" && node.snippet == "Self::new_unwrapped(bytes)" {
        calls.push(node);
    }
    for child in &node.children {
        helper_occurrences(child, calls);
    }
}

fn forge(pair: &Pair, attack: &str) -> Result<Pair, String> {
    let mut pdg = serde_json::to_value(&pair.pdg).map_err(|e| e.to_string())?;
    let mut cfg = serde_json::to_value(&pair.cfg).map_err(|e| e.to_string())?;
    match attack {
        "policy" => {
            downgrade(&mut pdg);
            downgrade(&mut cfg);
        }
        "product-control" => {
            let selector = id(&mut pdg, GUARD)?;
            flip_control(&mut cfg, &selector)?;
        }
        "stale-input" | "swapped-input" | "update-rhs" | "saved-result" => {
            forge_operand(&mut pdg, attack)?
        }
        "missing-update" | "duplicate-update" => forge_inventory(&mut pdg, attack)?,
        _ => forge_node(&mut pdg, attack)?,
    }
    Ok(Pair {
        pdg: serde_json::from_value(pdg).map_err(|e| e.to_string())?,
        cfg: serde_json::from_value(cfg).map_err(|e| e.to_string())?,
    })
}

fn node_mut<'a>(pdg: &'a mut Value, construct: &str) -> Result<&'a mut Value, String> {
    pdg["nodes"]
        .as_array_mut()
        .ok_or("Missing nodes")?
        .iter_mut()
        .find(|node| node["attributes"]["construct"] == construct)
        .ok_or_else(|| format!("Missing {construct} attack site"))
}

fn id(pdg: &mut Value, construct: &str) -> Result<Value, String> {
    Ok(node_mut(pdg, construct)?["id"].clone())
}

fn forge_operand(pdg: &mut Value, attack: &str) -> Result<(), String> {
    let target = id(
        pdg,
        if attack == "saved-result" {
            PAYLOAD
        } else {
            UPDATE
        },
    )?;
    let replacement = id(
        pdg,
        if attack == "stale-input" {
            PAYLOAD
        } else {
            "data.option-some"
        },
    )?;
    let edges = pdg["edges"].as_array_mut().ok_or("Missing edges")?;
    if attack == "swapped-input" {
        for edge in edges
            .iter_mut()
            .filter(|edge| edge["target"] == target && edge["subkind"] == "value_operand")
        {
            let position = edge["operand_position"]
                .as_u64()
                .ok_or("Missing position")?;
            edge["operand_position"] = json!(1 - position);
        }
    } else {
        let position = u64::from(attack == "update-rhs");
        let replacement = if attack == "update-rhs" {
            edges
                .iter()
                .find(|edge| edge["target"] == replacement && edge["subkind"] == "value_operand")
                .ok_or("Missing saved width")?["source"]
                .clone()
        } else {
            replacement
        };
        let edge = edges
            .iter_mut()
            .find(|edge| {
                edge["target"] == target
                    && edge["subkind"] == "value_operand"
                    && edge["operand_position"] == position
            })
            .ok_or("Missing attacked operand")?;
        edge["source"] = replacement;
    }
    Ok(())
}

fn forge_inventory(pdg: &mut Value, attack: &str) -> Result<(), String> {
    if attack == "missing-update" {
        node_mut(pdg, UPDATE)?["attributes"]["construct"] = json!("data.omitted-owned-update");
    } else {
        node_mut(pdg, "data.option-some")?["attributes"]["construct"] = json!(UPDATE);
    }
    Ok(())
}

fn forge_node(pdg: &mut Value, attack: &str) -> Result<(), String> {
    if attack == "wrong-owner" {
        let node = pdg["nodes"]
            .as_array_mut()
            .ok_or("Missing nodes")?
            .iter_mut()
            .find(|node| node["category"] == "assignment")
            .ok_or("Missing owner assignment")?;
        node["attributes"]["symbol_ids"] = json!(["forged-owner"]);
        return Ok(());
    }
    let construct = match attack {
        "product-guard" => GUARD,
        "product-payload" => PAYLOAD,
        "product-error" => ERROR,
        "ok-root" => "data.result-ok",
        "err-root" => "data.result-err",
        _ => UPDATE,
    };
    let node = node_mut(pdg, construct)?;
    match attack {
        "wrong-field" => node["attributes"]["literal"]["value"] = json!("0"),
        "fake-place" => node["annotations"]["rstyle.data:place-location"] = json!({}),
        "fake-memory" => {
            node["attributes"]["reads"] =
                json!([{"symbol_id":"forged-memory","operand_position":0}])
        }
        "product-guard" => {
            node["annotations"]["rstyle.data:product-result-propagation"]["true"] = json!("ok")
        }
        "product-payload" => {
            node["annotations"]["rstyle.data:product-result-propagation"]["requires"] = json!("err")
        }
        "product-error" => {
            node["annotations"]["rstyle.data:product-result-propagation"]["conversion"] =
                json!("unproved")
        }
        "ok-root" | "update-root" => node["annotations"][rstyle_pdg::data_origins::KEY] = json!([]),
        "err-root" => {
            node["annotations"][rstyle_pdg::data_origins::KEY] =
                json!([{"parameter":0,"path":[{"kind":"SharedReferent"}]}])
        }
        _ => return Err(format!("Unknown wrapped attack {attack}")),
    }
    Ok(())
}

fn flip_control(cfg: &mut Value, selector: &Value) -> Result<(), String> {
    let edges = cfg["edges"].as_array_mut().ok_or("Missing CFG edges")?;
    let mut changed = 0;
    for edge in edges.iter_mut().filter(|edge| edge["source"] == *selector) {
        match edge["kind"].as_str() {
            Some("true") => {
                edge["kind"] = json!("false");
                changed += 1;
            }
            Some("false") => {
                edge["kind"] = json!("true");
                changed += 1;
            }
            _ => {}
        }
    }
    if changed != 2 {
        return Err("Product selector lacks exact two branch outcomes".into());
    }
    Ok(())
}

fn downgrade(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            if fields.get("id").and_then(Value::as_str) == Some("rstyle.rust.hir-data")
                && fields.contains_key("version")
            {
                fields.insert("version".into(), json!("3.89.0"));
            }
            for child in fields.values_mut() {
                downgrade(child);
            }
        }
        Value::Array(values) => {
            for child in values {
                downgrade(child);
            }
        }
        _ => {}
    }
}
