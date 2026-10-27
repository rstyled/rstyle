//! Exact whole reader operations; ordinary memory/location verification stays active.
use super::*;
use crate::CfgEdgeKind;
mod flow;

pub(super) struct Operations<'a> {
    pub inputs: [&'a PdgNode; 2],
    pub arguments: [&'a PdgNode; 3],
    pub length: &'a PdgNode,
    pub selector: &'a PdgNode,
    pub error: &'a PdgNode,
    pub error_return: &'a PdgNode,
    pub unit: &'a PdgNode,
    pub reader_argument: &'a PdgNode,
    pub reader: &'a PdgNode,
    pub consumed: &'a PdgNode,
    pub tuple: &'a PdgNode,
    pub success: &'a PdgNode,
    pub returned: &'a PdgNode,
}

pub(super) fn verify(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    source: &source::Checked<'_>,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    if source.width == crate::native_reader_width::Width::U16 {
        crate::extraction_profile::require_document(
            &pair.pdg.profile,
            crate::native_reader_width::CAPABILITY,
            || true,
            "Native u16 reader requires its registered complete document policy",
        )?;
    }
    let operations = operations(pair, facts, source)?;
    for node in &pair.pdg.nodes {
        if !node.attributes.writes.is_empty()
            || node.attributes.write_form.is_some()
            || node.annotations.contains_key(crate::data_places::KEY)
            || node.id != operations.reader.id
                && node
                    .attributes
                    .reads
                    .iter()
                    .any(|access| access.location_id.is_some())
        {
            return Err(
                "Fallible reader invents memory writes or content reads outside the native helper"
                    .into(),
            );
        }
    }

    super::super::named_error_result_propagation::verify(pair, facts)?;
    crate::helper_calls::verify_documents(pair, facts)?;
    operands(pair, &operations)?;
    flow::verify(pair, &operations)?;
    guarded(pair, &operations)?;

    let expected = origins::derive(facts, source, &operations)?;
    if expected.len() != pair.pdg.nodes.len()
        || pair
            .pdg
            .nodes
            .iter()
            .any(|node| !expected.contains_key(&node.id))
    {
        return Err(
            "Fallible reader graph contains omitted, duplicate or unaccounted operations".into(),
        );
    }
    Ok(expected)
}

fn operations<'a>(
    pair: PairView<'a>,
    facts: &dyn EvidenceFacts,
    source: &source::Checked<'_>,
) -> Result<Operations<'a>, String> {
    let propagation = &source.propagation;
    let constructs = propagation.mode.constructs();
    Ok(Operations {
        inputs: [input(pair, facts, 0)?, input(pair, facts, 1)?],
        arguments: [
            value_operation(pair, source.length_arguments[0])?,
            value_operation(pair, source.length_arguments[1])?,
            value_operation(pair, source.length_arguments[2])?,
        ],
        length: value_operation(pair, propagation.operand)?,
        selector: typed_operation(pair, source.question, constructs[0], "bool")?,
        error: typed_operation(
            pair,
            source.question,
            constructs[1],
            &propagation.output_token,
        )?,
        error_return: serialized_operation(pair, &propagation.early_return.span, |node| {
            node.category == NodeCategory::Return
        })?,
        unit: typed_operation(pair, source.question, constructs[2], "unit")?,
        reader_argument: value_operation(pair, source.reader_argument)?,
        reader: value_operation(pair, source.reader)?,
        consumed: value_operation(pair, source.consumed)?,
        tuple: value_operation(pair, source.tuple)?,
        success: typed_operation(
            pair,
            source.success,
            "data.result-ok",
            &propagation.output_token,
        )?,
        returned: serialized_operation(pair, &source.success.span, |node| {
            node.category == NodeCategory::Return
        })?,
    })
}

fn input<'a>(
    pair: PairView<'a>,
    facts: &dyn EvidenceFacts,
    position: usize,
) -> Result<&'a PdgNode, String> {
    let node = serialized_operation(pair, &facts.signature.params[position].pat.span, |node| {
        node.category == NodeCategory::FormalInput
    })?;
    if node.attributes.position != Some(position as u64)
        || node
            .annotations
            .get("compat.rust.hir-r0:type")
            .and_then(|value| value.get("value"))
            .and_then(serde_json::Value::as_str)
            != facts.signature.params[position].ty.normalized.as_deref()
    {
        return Err("Reader input changes original formal position".into());
    }
    Ok(node)
}

fn operands(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for (input, reads) in [
        (ops.inputs[0], vec![ops.arguments[0], ops.reader_argument]),
        (ops.inputs[1], vec![ops.arguments[2]]),
    ] {
        for read in &reads {
            super::super::copy_tagged_values::origins::verify_definition(pair, input, read)?;
        }
        super::super::value_consumers::verify(pair, input, &reads, "reaching_definition")?;
    }

    verify_saved_operands(pair, ops.length, &ops.arguments)?;
    verify_saved_operands(pair, ops.reader, &[ops.reader_argument])?;
    verify_saved_operands(pair, ops.tuple, &[ops.reader, ops.consumed])?;
    verify_saved_operands(pair, ops.success, &[ops.tuple])?;
    verify_saved_operands(pair, ops.returned, &[ops.success])?;

    for (value, target) in [
        (ops.arguments[0], ops.length),
        (ops.arguments[1], ops.length),
        (ops.arguments[2], ops.length),
        (ops.reader_argument, ops.reader),
        (ops.reader, ops.tuple),
        (ops.consumed, ops.tuple),
        (ops.tuple, ops.success),
        (ops.success, ops.returned),
    ] {
        super::super::value_consumers::verify(pair, value, &[target], "value_operand")?;
    }

    super::super::value_consumers::verify(pair, ops.unit, &[], "value_operand")?;
    super::super::value_consumers::verify(pair, ops.returned, &[], "value_operand")?;
    super::super::value_consumers::verify(pair, ops.error_return, &[], "value_operand")?;
    Ok(())
}

fn guarded(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    let guards = super::super::conditional_payloads::Guards::new(pair)?;
    for node in [
        ops.reader_argument,
        ops.reader,
        ops.consumed,
        ops.tuple,
        ops.success,
        ops.returned,
    ] {
        guards.guarded(ops.selector, node, CfgEdgeKind::False)?;
    }

    super::super::callbacks::order::sequence(
        pair,
        &[
            ops.arguments[0],
            ops.arguments[1],
            ops.arguments[2],
            ops.length,
            ops.selector,
        ],
    )?;

    super::super::callbacks::order::sequence(
        pair,
        &[
            ops.unit,
            ops.reader_argument,
            ops.reader,
            ops.consumed,
            ops.tuple,
            ops.success,
            ops.returned,
        ],
    )?;
    successful_return(pair, ops)
}

fn successful_return(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    let return_form = ops
        .returned
        .attributes
        .extensions
        .get("compat.rust.hir-r0:return-form")
        .and_then(serde_json::Value::as_str);
    if ops.returned.category != NodeCategory::Return
        || return_form != Some("implicit")
        || ops
            .returned
            .annotations
            .get("compat.rust.hir-r0:type")
            .and_then(|value| value.get("value"))
            .and_then(serde_json::Value::as_str)
            != ops
                .success
                .annotations
                .get("compat.rust.hir-r0:type")
                .and_then(|value| value.get("value"))
                .and_then(serde_json::Value::as_str)
    {
        return Err("Reader loses its original implicit successful return".into());
    }
    super::super::result_propagation::return_to_exit(pair, ops.returned)
}
