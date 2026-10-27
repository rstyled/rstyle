//! The None-owned error is unreachable on the successful checked arithmetic path.
use super::*;
use crate::CfgEdgeKind;

pub(in super::super) struct Operations<'a> {
    pub inputs: [&'a PdgNode; 3],
    pub left: &'a PdgNode,
    pub right: &'a PdgNode,
    pub checked: &'a PdgNode,
    pub some: &'a PdgNode,
    pub none: &'a PdgNode,
    pub payload: &'a PdgNode,
    pub binding: &'a PdgNode,
    pub read: &'a PdgNode,
    pub ok: &'a PdgNode,
    pub actual: &'a PdgNode,
    pub call: &'a PdgNode,
    pub err: &'a PdgNode,
    pub merge: &'a PdgNode,
    pub returned: &'a PdgNode,
}

pub(super) fn verify(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    root: &source::Root<'_>,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    let ops = operations(pair, facts, root)?;
    operands(pair, root, &ops)?;
    guards(pair, root, &ops)?;
    merge(pair, root, &ops)?;

    super::super::super::copy_tagged_values::graph::returned(pair, root.expression, ops.merge)?;
    origins::arithmetic(&ops, root, &facts.signature.params[2].pat.hir)
}

fn operations<'a>(
    pair: PairView<'a>,
    facts: &dyn EvidenceFacts,
    root: &source::Root<'_>,
) -> Result<Operations<'a>, String> {
    let expression = root.expression;
    let some = &expression.children[root.option.some_arm + 1].children[0];
    let none = &expression.children[root.option.none_arm + 1].children[0];
    Ok(Operations {
        inputs: [
            serialized_formal_input(pair, &facts.signature.params[0].pat.span, 0)?,
            serialized_formal_input(pair, &facts.signature.params[1].pat.span, 1)?,
            serialized_formal_input(pair, &facts.signature.params[2].pat.span, 2)?,
        ],
        left: value_operation(pair, &root.option.scrutinee.children[0])?,
        right: value_operation(pair, &root.option.scrutinee.children[1])?,
        checked: operation(pair, root.option.scrutinee, root.kind.construct())?,
        some: operation(pair, some, "match.option-some")?,
        none: operation(pair, none, "match.option-none")?,
        payload: operation(pair, root.option.binding, "data.option-value-payload")?,
        binding: serialized_operation(pair, &root.option.binding.span, |node| {
            node.category == NodeCategory::Declaration
        })?,
        read: value_operation(pair, &root.ok.children[0])?,
        ok: operation(pair, root.ok, "data.result-ok")?,
        actual: value_operation(pair, root.actual)?,
        call: value_operation(pair, root.call)?,
        err: operation(pair, root.err, "data.result-err")?,
        merge: value_operation(pair, expression)?,
        returned: returned(pair, expression)?,
    })
}

fn operands(
    pair: PairView<'_>,
    root: &source::Root<'_>,
    ops: &Operations<'_>,
) -> Result<(), String> {
    definitions(pair, ops)?;
    verify_saved_operands(pair, ops.checked, &[ops.left, ops.right])?;

    for target in [ops.some, ops.none, ops.payload] {
        verify_saved_operands(pair, target, &[ops.checked])?;
    }
    super::super::super::conditional_payloads::binding(pair, root.option.binding, ops.payload)?;
    super::super::super::copy_tagged_values::origins::verify_definition(
        pair,
        ops.binding,
        ops.read,
    )?;

    for (earlier, later) in [
        (ops.read, ops.ok),
        (ops.actual, ops.call),
        (ops.call, ops.err),
    ] {
        edge(pair, earlier, later)?;
        consumers(pair, earlier, &[later], "value_operand")?;
    }
    value_consumers(pair, ops)?;

    for (earlier, later) in [
        (ops.left, ops.right),
        (ops.right, ops.checked),
        (ops.payload, ops.binding),
        (ops.binding, ops.read),
    ] {
        before(pair, earlier, later)?;
    }
    call_effects(ops.call)
}

fn definitions(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for (input, read) in [
        (ops.inputs[0], ops.left),
        (ops.inputs[1], ops.right),
        (ops.inputs[2], ops.actual),
    ] {
        super::super::super::copy_tagged_values::origins::verify_definition(pair, input, read)?;
        consumers(pair, input, &[read], "reaching_definition")?;
    }
    Ok(())
}

fn value_consumers(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for value in [ops.left, ops.right] {
        consumers(pair, value, &[ops.checked], "value_operand")?;
    }
    consumers(
        pair,
        ops.checked,
        &[ops.some, ops.none, ops.payload],
        "value_operand",
    )?;
    consumers(pair, ops.payload, &[ops.binding], "value_operand")?;
    consumers(pair, ops.binding, &[ops.read], "reaching_definition")?;

    for value in [ops.ok, ops.err] {
        consumers(pair, value, &[ops.merge], "value_alternative")?;
    }
    Ok(())
}

fn call_effects(call: &PdgNode) -> Result<(), String> {
    if call.category != NodeCategory::Call
        || call.attributes.call_form != Some(crate::CallForm::Direct)
        || !call.attributes.reads.is_empty()
        || !call.attributes.writes.is_empty()
        || !call.attributes.defines.is_empty()
        || !call.attributes.symbol_ids.is_empty()
        || call.attributes.literal.is_some()
    {
        return Err("Fresh error helper call invents effects or changes dispatch".into());
    }
    Ok(())
}

fn guards(pair: PairView<'_>, root: &source::Root<'_>, ops: &Operations<'_>) -> Result<(), String> {
    let guards = super::super::super::conditional_payloads::Guards::new(pair)?;
    for node in [ops.payload, ops.binding, ops.read, ops.ok] {
        guards.guarded(ops.some, node, CfgEdgeKind::True)?;
    }
    for node in [ops.actual, ops.call, ops.err] {
        guards.guarded(ops.none, node, CfgEdgeKind::True)?;
    }

    let (first, second) = if root.option.some_arm == 0 {
        (ops.some, ops.none)
    } else {
        (ops.none, ops.some)
    };
    guards.guarded(first, second, CfgEdgeKind::False)
}

fn merge(pair: PairView<'_>, root: &source::Root<'_>, ops: &Operations<'_>) -> Result<(), String> {
    let site = super::super::super::callbacks::order::site(pair, &ops.merge.id)?;
    let alternatives = site["alternatives"]
        .as_array()
        .ok_or("Missing arithmetic alternatives")?;
    let first = if root.option.some_arm == 0 {
        ops.some
    } else {
        ops.none
    };
    if ops.merge.category != NodeCategory::Merge
        || alternatives.len() != 2
        || site["controller_pdg_node_id"] != first.id
        || site["rstyle.hir-coverage:selection-kind"] != "ordered-match/1"
    {
        return Err("Fresh arithmetic merge changes its original Option selection".into());
    }

    verify_saved_operands(pair, ops.merge, &[])?;
    for (index, value) in [
        (root.option.some_arm, ops.ok),
        (root.option.none_arm, ops.err),
    ] {
        let completion = super::super::super::callbacks::order::completion(pair, value)?;
        if alternatives
            .iter()
            .filter(|alternative| {
                alternative["outcome"] == format!("arm:{index}")
                    && alternative["value_pdg_node_id"] == value.id
                    && alternative["completion_cfg_node_ids"] == serde_json::json!(completion)
            })
            .count()
            != 1
        {
            return Err("Fresh arithmetic merge changes a complete branch result".into());
        }
    }

    let incoming = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| edge.target == ops.merge.id && edge.kind == crate::EdgeFamily::Data)
        .count();
    if incoming != 2 {
        return Err("Fresh arithmetic merge contains unrelated alternatives".into());
    }
    Ok(())
}
