//! Every original operation participates in the short-input or selected-reader path.
mod flow;
mod origins;

use super::*;
use serde_json::Value;

pub(super) struct Operations<'a> {
    pub inputs: [&'a PdgNode; 2],
    pub slice: &'a PdgNode,
    pub length: &'a PdgNode,
    pub width: &'a PdgNode,
    pub condition: &'a PdgNode,
    pub predicate: &'a PdgNode,
    pub message: &'a PdgNode,
    pub constructor: &'a PdgNode,
    pub error: &'a PdgNode,
    pub error_return: &'a PdgNode,
    pub reader_actuals: [&'a PdgNode; 2],
    pub reader: &'a PdgNode,
    pub returned: &'a PdgNode,
}

pub(super) fn verify(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    checked: &source::Checked<'_>,
) -> Result<BTreeMap<String, Value>, String> {
    let ops = operations(pair, facts, checked)?;
    operands(pair, &ops)?;
    definitions(pair, &ops)?;
    flow::verify(pair, &ops)?;
    guards(pair, &ops)?;
    effects(pair, &ops)?;

    crate::helper_calls::verify_documents(pair, facts)?;

    let roles = origins::derive(checked, &ops)?;

    if roles.len() != pair.pdg.nodes.len()
        || pair
            .pdg
            .nodes
            .iter()
            .any(|node| !roles.contains_key(&node.id))
    {
        return Err(
            "Checked ID reader contains omitted, duplicate or unaccounted operations".into(),
        );
    }

    Ok(roles)
}

fn operations<'a>(
    pair: PairView<'a>,
    facts: &dyn EvidenceFacts,
    checked: &source::Checked<'_>,
) -> Result<Operations<'a>, String> {
    Ok(Operations {
        inputs: [
            super::super::serialized_formal_input(
                pair,
                &facts.signature.params[0].pat.span,
                u64::from(facts.signature.params[0].position),
            )?,
            super::super::serialized_formal_input(
                pair,
                &facts.signature.params[1].pat.span,
                u64::from(facts.signature.params[1].position),
            )?,
        ],
        slice: super::super::value_operation(pair, checked.slice_read)?,
        length: super::super::value_operation(pair, checked.length)?,
        width: super::super::value_operation(pair, checked.width)?,
        condition: super::super::value_operation(pair, checked.condition)?,
        predicate: super::super::typed_operation(pair, checked.condition, "if", "bool")?,
        message: super::super::value_operation(pair, checked.message)?,
        constructor: super::super::value_operation(pair, checked.constructor)?,
        error: super::super::value_operation(pair, checked.error_result)?,
        error_return: returned(pair, checked.error_return)?,
        reader_actuals: [
            super::super::value_operation(pair, checked.reader_actuals[0])?,
            super::super::value_operation(pair, checked.reader_actuals[1])?,
        ],
        reader: super::super::value_operation(pair, checked.reader)?,
        returned: returned(pair, checked.reader)?,
    })
}

fn returned<'a>(pair: PairView<'a>, source: &HirNode) -> Result<&'a PdgNode, String> {
    super::super::serialized_operation(pair, &source.span, |node| {
        node.category == NodeCategory::Return
    })
}

fn operands(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for (node, values) in [
        (ops.length, vec![ops.slice]),
        (ops.condition, vec![ops.length, ops.width]),
        (ops.predicate, vec![ops.condition]),
        (ops.constructor, vec![ops.message]),
        (ops.error, vec![ops.constructor]),
        (ops.error_return, vec![ops.error]),
        (ops.reader, ops.reader_actuals.to_vec()),
        (ops.returned, vec![ops.reader]),
    ] {
        super::super::verify_saved_operands(pair, node, &values)?;

        for value in values {
            super::super::value_consumers::verify(pair, value, &[node], "value_operand")?;
        }
    }

    for terminal in [ops.error_return, ops.returned, ops.predicate] {
        super::super::value_consumers::verify(pair, terminal, &[], "value_operand")?;
    }

    Ok(())
}

fn definitions(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for (input, reads) in [
        (ops.inputs[0], vec![ops.slice, ops.reader_actuals[0]]),
        (ops.inputs[1], vec![ops.message, ops.reader_actuals[1]]),
    ] {
        for read in &reads {
            super::super::copy_tagged_values::origins::verify_definition(pair, input, read)?;
        }

        super::super::value_consumers::verify(pair, input, &reads, "reaching_definition")?;
    }

    Ok(())
}

fn guards(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    let guards = super::super::conditional_payloads::Guards::new(pair)?;

    for node in [ops.message, ops.constructor, ops.error, ops.error_return] {
        guards.guarded(ops.predicate, node, crate::CfgEdgeKind::True)?;
    }

    for node in [
        ops.reader_actuals[0],
        ops.reader_actuals[1],
        ops.reader,
        ops.returned,
    ] {
        guards.guarded(ops.predicate, node, crate::CfgEdgeKind::False)?;
    }

    ordered_paths(pair, ops)
}

fn ordered_paths(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for sequence in [
        vec![
            ops.slice,
            ops.length,
            ops.width,
            ops.condition,
            ops.predicate,
        ],
        vec![ops.message, ops.constructor, ops.error, ops.error_return],
        vec![
            ops.reader_actuals[0],
            ops.reader_actuals[1],
            ops.reader,
            ops.returned,
        ],
    ] {
        super::super::callbacks::order::sequence(pair, &sequence)?;
    }

    for returned in [ops.error_return, ops.returned] {
        super::super::result_propagation::return_to_exit(pair, returned)?;
    }

    Ok(())
}

fn effects(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    if pair.cfg.extraction_evidence.status != crate::ExtractionStatus::Partial {
        return Err(
            "Checked ID reader must preserve the selected reader's actual possible panics".into(),
        );
    }

    for node in &pair.pdg.nodes {
        if !node.attributes.writes.is_empty()
            || node.attributes.write_form.is_some()
            || node.annotations.contains_key(crate::data_places::KEY)
            || node.id != ops.reader.id
                && node
                    .attributes
                    .reads
                    .iter()
                    .any(|read| read.location_id.is_some())
        {
            return Err(
                "Checked ID reader invents writes or reads outside its guarded selected reader"
                    .into(),
            );
        }
    }

    let comparison = ops.condition;

    if comparison.category != NodeCategory::Computation
        || comparison
            .attributes
            .operator
            .as_ref()
            .and_then(|value| value.known_value())
            != Some("<")
        || comparison.attributes.literal.is_some()
        || comparison.attributes.callee.is_some()
        || !comparison.attributes.reads.is_empty()
        || !comparison.attributes.defines.is_empty()
    {
        return Err("Checked ID reader comparison changes builtin unsigned less-than".into());
    }

    super::super::verify_value_effects(
        ops.predicate,
        false,
        "Checked ID predicate invents effects",
    )?;

    if ops.predicate.category != NodeCategory::Control {
        return Err("Checked ID reader loses its exact short-input predicate".into());
    }

    Ok(())
}
