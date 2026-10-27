//! Err guards, single-use transfers and exact two-branch joins are independent checks.
use super::*;

pub(super) fn verify(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    exact_flow(pair, ops)?;

    let guards = super::super::conditional_payloads::Guards::new(pair)?;

    for node in [
        ops.payload,
        ops.declaration,
        ops.error_read,
        ops.message_read,
        ops.constructor,
        ops.error,
    ] {
        guards.guarded(ops.selector, node, crate::CfgEdgeKind::True)?;
    }

    for node in [ops.success_payload, ops.success] {
        guards.guarded(ops.selector, node, crate::CfgEdgeKind::False)?;
    }

    for sequence in [
        vec![ops.receiver, ops.environment, ops.selector],
        vec![
            ops.payload,
            ops.declaration,
            ops.error_read,
            ops.message_read,
            ops.constructor,
            ops.error,
        ],
        vec![ops.success_payload, ops.success],
    ] {
        super::super::callbacks::order::sequence(pair, &sequence)?;
    }
    merge(pair, ops)
}

fn exact_flow(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    use crate::CfgEdgeKind::{False, Normal, True};

    let flow = crate::slice_patterns::graph::flow::Flow::new(pair)?;
    flow.outgoing(ops.receiver, &[(Normal, ops.environment)])?;
    flow.outgoing(ops.environment, &[(Normal, ops.selector)])?;
    flow.outgoing(
        ops.selector,
        &[(True, ops.payload), (False, ops.success_payload)],
    )?;

    for chain in [
        vec![
            ops.payload,
            ops.declaration,
            ops.error_read,
            ops.message_read,
            ops.constructor,
            ops.error,
        ],
        vec![ops.success_payload, ops.success],
    ] {
        for adjacent in chain.windows(2) {
            flow.outgoing(adjacent[0], &[(Normal, adjacent[1])])?;
        }
    }
    completions(pair, ops, &flow)
}

fn completions(
    pair: PairView<'_>,
    ops: &Operations<'_>,
    flow: &crate::slice_patterns::graph::flow::Flow<'_>,
) -> Result<(), String> {
    let site = super::super::callbacks::order::site(pair, &ops.merge.id)?;

    let successor = site["cfg_successor_node_id"]
        .as_str()
        .ok_or("ID map lacks real continuation")?;

    let mapping = crate::read_operation_map(&pair.pdg.extensions)?;

    let entries: Vec<_> = mapping
        .iter()
        .filter(|entry| entry.cfg_node_id == successor)
        .collect();

    let [entry] = entries.as_slice() else {
        return Err("ID map continuation requires one real mapped operation".into());
    };

    let continuation = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.id == entry.pdg_node_id)
        .ok_or("ID map continuation has no original graph value")?;

    let alternatives = site["alternatives"]
        .as_array()
        .ok_or("Missing ID map completions")?;

    for (outcome, wrapped) in [("true", ops.error), ("false", ops.success)] {
        flow.outgoing(wrapped, &[(crate::CfgEdgeKind::Normal, continuation)])?;

        let alternative = alternatives
            .iter()
            .find(|alternative| alternative["outcome"] == outcome)
            .ok_or("Missing ID map selected completion")?;

        if alternative["completion_cfg_node_ids"] != serde_json::json!([flow.operation(wrapped)?]) {
            return Err("ID map completion frontier bypasses its exact wrapped Result".into());
        }
    }

    Ok(())
}

fn merge(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    let incoming: Vec<_> = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| edge.target == ops.merge.id && edge.kind == crate::EdgeFamily::Data)
        .collect();

    if incoming.len() != 2
        || incoming.iter().any(|edge| {
            edge.subkind != "value_alternative"
                || ![ops.error.id.as_str(), ops.success.id.as_str()].contains(&edge.source.as_str())
        })
    {
        return Err("ID error map merge invents an unrelated result alternative".into());
    }

    let site = super::super::callbacks::order::site(pair, &ops.merge.id)?;

    let alternatives = site["alternatives"]
        .as_array()
        .ok_or("Missing ID map alternatives")?;

    if alternatives.len() != 2 || site["controller_pdg_node_id"] != ops.selector.id {
        return Err("ID error map loses its exact saved discriminator".into());
    }

    for (outcome, value) in [("true", ops.error), ("false", ops.success)] {
        if alternatives
            .iter()
            .filter(|alternative| {
                alternative["outcome"] == outcome && alternative["value_pdg_node_id"] == value.id
            })
            .count()
            != 1
        {
            return Err("ID error map changes the selected error/success branch".into());
        }

        super::super::value_consumers::verify(pair, value, &[ops.merge], "value_alternative")?;
    }

    Ok(())
}
