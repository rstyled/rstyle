//! Exact branch values, consumers, merge alternatives, and final return.
use super::*;
use crate::CfgEdgeKind;

pub(super) struct Operations<'a> {
    pub saved: &'a PdgNode,
    pub selector: &'a PdgNode,
    pub payload: &'a PdgNode,
    pub binding: &'a PdgNode,
    pub read: &'a PdgNode,
    pub wildcard: &'a PdgNode,
    pub ok: &'a PdgNode,
    pub error: &'a PdgNode,
    pub err: &'a PdgNode,
    pub merge: &'a PdgNode,
}

pub(super) fn inverse(
    pair: PairView<'_>,
    source: &HirNode,
    checked: &crate::copy_tagged_values::Match<'_>,
) -> Result<(), String> {
    let ops = Operations {
        saved: value_operation(pair, checked.scrutinee)?,
        selector: operation(pair, checked.pattern, CONSTRUCTS[1], Some(&checked.tag))?,
        payload: operation(pair, checked.binding, CONSTRUCTS[2], Some(&checked.tag))?,
        binding: serialized_operation(pair, &checked.binding.span, |node| {
            node.category == NodeCategory::Declaration
        })?,
        read: value_operation(pair, &checked.success.children[0])?,
        wildcard: operation(pair, checked.wildcard, CONSTRUCTS[3], None)?,
        ok: operation(pair, checked.success, CONSTRUCTS[4], None)?,
        error: operation(pair, &checked.failure.children[0], CONSTRUCTS[6], None)?,
        err: operation(pair, checked.failure, CONSTRUCTS[5], None)?,
        merge: value_operation(pair, source)?,
    };
    verify(pair, &ops)?;
    returned(pair, source, ops.merge)
}

pub(super) fn verify(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    consumers(
        pair,
        ops.saved,
        &[ops.selector, ops.payload],
        "value_operand",
    )?;

    for (value, consumer, kind) in [
        (ops.payload, ops.binding, "value_operand"),
        (ops.binding, ops.read, "reaching_definition"),
        (ops.read, ops.ok, "value_operand"),
        (ops.error, ops.err, "value_operand"),
        (ops.ok, ops.merge, "value_alternative"),
        (ops.err, ops.merge, "value_alternative"),
    ] {
        consumers(pair, value, &[consumer], kind)?;
    }
    branch_guards(pair, ops)?;

    for (earlier, later) in [
        (ops.saved, ops.selector),
        (ops.payload, ops.binding),
        (ops.binding, ops.read),
        (ops.read, ops.ok),
        (ops.wildcard, ops.error),
        (ops.error, ops.err),
    ] {
        super::super::callbacks::order::before(
            pair,
            &super::super::callbacks::order::completion(pair, earlier)?,
            later,
        )?;
    }

    merge(pair, ops)
}

fn branch_guards(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    let guards = super::super::conditional_payloads::Guards::new(pair)?;
    for node in [ops.payload, ops.binding, ops.read, ops.ok] {
        guards.guarded(ops.selector, node, CfgEdgeKind::True)?;
    }
    for node in [ops.wildcard, ops.error, ops.err] {
        guards.guarded(ops.selector, node, CfgEdgeKind::False)?;
    }
    Ok(())
}

pub(super) use super::super::value_consumers::verify as consumers;

fn merge(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    let site = super::super::callbacks::order::site(pair, &ops.merge.id)?;
    let alternatives = site["alternatives"]
        .as_array()
        .ok_or("Missing conversion alternatives")?;
    if ops.merge.category != NodeCategory::Merge
        || alternatives.len() != 2
        || site["controller_pdg_node_id"] != ops.selector.id
        || site["rstyle.hir-coverage:selection-kind"] != "ordered-match/1"
    {
        return Err("Copy conversion merge changes its ordered original selector".into());
    }

    verify_saved_operands(pair, ops.merge, &[])?;
    for (outcome, value) in [("arm:0", ops.ok), ("arm:1", ops.err)] {
        let completed = super::super::callbacks::order::completion(pair, value)?;
        if alternatives
            .iter()
            .filter(|alternative| {
                alternative["outcome"] == outcome
                    && alternative["value_pdg_node_id"] == value.id
                    && alternative["completion_cfg_node_ids"] == serde_json::json!(completed)
            })
            .count()
            != 1
        {
            return Err("Copy conversion merge changes an exact original branch result".into());
        }
    }

    let incoming = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| edge.target == ops.merge.id && edge.kind == crate::EdgeFamily::Data)
        .count();
    if incoming != 2 {
        return Err("Copy conversion merge contains unrelated alternatives".into());
    }
    Ok(())
}

pub(super) fn returned(
    pair: PairView<'_>,
    source: &HirNode,
    value: &PdgNode,
) -> Result<(), String> {
    let returned = serialized_operation(pair, &source.span, |node| {
        node.category == NodeCategory::Return
    })?;
    verify_saved_operands(pair, returned, &[value])?;
    consumers(pair, value, &[returned], "value_operand")?;
    super::super::verify_value_effects(returned, false, "Copy conversion return invents effects")?;

    if pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.category == NodeCategory::Return)
        .count()
        != 1
        || returned
            .attributes
            .extensions
            .get("compat.rust.hir-r0:return-form")
            .and_then(serde_json::Value::as_str)
            != Some("implicit")
        || returned.annotations.get("compat.rust.hir-r0:type")
            != value.annotations.get("compat.rust.hir-r0:type")
    {
        return Err("Copy conversion must return its complete final value exactly once".into());
    }

    return_frontier(pair, value, returned)?;
    super::super::result_propagation::return_to_exit(pair, returned)
}

fn return_frontier(pair: PairView<'_>, value: &PdgNode, returned: &PdgNode) -> Result<(), String> {
    let targets = super::super::callbacks::order::completion(pair, returned)?;
    let completed = super::super::callbacks::order::completion(pair, value)?;
    let incoming: Vec<_> = pair
        .cfg
        .edges
        .iter()
        .filter(|edge| targets.contains(&edge.target))
        .collect();

    if incoming.len() != completed.len()
        || completed.iter().any(|id| {
            incoming
                .iter()
                .filter(|edge| edge.source == *id && edge.kind == CfgEdgeKind::Normal)
                .count()
                != 1
        })
    {
        return Err("Copy conversion return loses its exact branch completion frontier".into());
    }

    if value.category == NodeCategory::Merge {
        let site = super::super::callbacks::order::site(pair, &value.id)?;
        if !site["cfg_successor_node_id"]
            .as_str()
            .is_some_and(|id| targets.contains(id))
        {
            return Err("Copy conversion merge does not complete at its final return".into());
        }
    }
    Ok(())
}
