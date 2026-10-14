//! Independent serialized address computation and origin checks.

use crate::{NodeCategory, PdgNode, data_origins::Origin};

pub(super) fn verify(
    pair: crate::producer::PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    let places = crate::data_places::Places::verify(facts)?;
    for node in &pair.pdg.nodes {
        if node.annotations.contains_key(crate::data_places::KEY) {
            let source = super::origins::source_node(node, facts)?;
            if !places.contains(source) {
                return Err("Orphan serialized place interpretation".into());
            }
        }
        if node.category == NodeCategory::Return
            || node.original_kind.known_value() != Some("expr_addr_of")
        {
            continue;
        }
        verify_address(node)?;
        let source = super::origins::source_node(node, facts)?;
        let expected = crate::data_places::borrow_origin(source, facts)?;
        let operand = super::field_base(pair, facts, node)?;
        super::verify_field_value_edge(pair, node, operand)?;
        if graph_origin(node, pair)? != expected {
            return Err("Borrow graph origin disagrees with original field location".into());
        }
    }
    Ok(())
}

pub(super) fn verify_place(node: &PdgNode, location: &str) -> Result<(), String> {
    if node.category != NodeCategory::Computation
        || node.attributes.construct.as_deref() != Some("data.place")
        || node
            .annotations
            .get(crate::data_places::KEY)
            .and_then(serde_json::Value::as_str)
            != Some(location)
    {
        return Err("Serialized place role/location disagrees with source".into());
    }
    verify_effects(node)
}

fn verify_address(node: &PdgNode) -> Result<(), String> {
    if node.category != NodeCategory::Computation
        || node.attributes.construct.as_deref() != Some("data.borrow")
    {
        return Err("Serialized borrow loses source-backed address computation".into());
    }
    verify_effects(node)
}

fn verify_effects(node: &PdgNode) -> Result<(), String> {
    if node.attributes.operator.is_some()
        || node.attributes.literal.is_some()
        || node.attributes.callee.is_some()
        || node.attributes.call_form.is_some()
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
    {
        return Err("Address computation invents value reads, writes or definitions".into());
    }
    Ok(())
}

pub(super) fn graph_origin(
    node: &PdgNode,
    pair: crate::producer::PairView<'_>,
) -> Result<Origin, String> {
    let operands: Vec<_> = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| {
            edge.target == node.id
                && edge.kind == crate::EdgeFamily::Data
                && edge.subkind == "value_operand"
        })
        .collect();
    let [edge] = operands.as_slice() else {
        return Err("Borrow must retain exactly one place operand".into());
    };
    if edge.operand_position != Some(0) {
        return Err("Borrow operand evaluation position disagrees".into());
    }

    let place = pair
        .pdg
        .nodes
        .iter()
        .find(|candidate| candidate.id == edge.source)
        .ok_or("Missing borrow operand")?;
    let id = place
        .annotations
        .get(crate::data_places::KEY)
        .and_then(serde_json::Value::as_str)
        .ok_or("Borrow operand is not a verified place")?;

    let location = pair
        .pdg
        .memory_locations
        .iter()
        .find(|location| location.id == id)
        .ok_or("Borrow place lacks exact location evidence")?;
    let evidence = location
        .extensions
        .get(super::LOCATION_KEY)
        .ok_or("Missing borrow formal location interpretation")?;
    serde_json::from_value(
        serde_json::json!({"parameter": evidence["parameter"], "path": evidence["path"]}),
    )
    .map_err(|error| error.to_string())
}
