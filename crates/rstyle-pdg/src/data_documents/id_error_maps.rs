//! Replay immediate error callbacks without granting surrounding reader coverage.
mod flow;
mod operations;
mod source;

use super::*;
use crate::{id_error_maps as contract, unit_store::EvidenceFacts};
pub(crate) use operations::{Operations, operations};
pub(crate) use source::{source_node, verify_type};

pub(crate) const CONSTRUCTS: [&str; 9] = [
    contract::ENVIRONMENT,
    contract::SELECTOR,
    contract::PAYLOAD,
    contract::BINDING,
    contract::MESSAGE,
    contract::SUCCESS,
    contract::ERROR,
    contract::OK,
    contract::RESULT,
];

pub(crate) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let sources = contract::sources(facts);

    super::verify_construct_inventory(pair, CONSTRUCTS, [sources.len(); 9])?;

    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        contract::CAPABILITY,
        || !sources.is_empty(),
        "ID error maps require their registered document policy",
    )?;

    for source in sources {
        let checked = contract::verify(source, facts)?;

        let ops = operations(pair, &checked)?;
        effects(&ops, &checked)?;
        operands(pair, &ops)?;
        binding(pair, facts, &checked, &ops)?;
        flow::verify(pair, &ops)?;
    }

    Ok(())
}

fn effects(ops: &Operations<'_>, checked: &contract::Checked<'_>) -> Result<(), String> {
    for node in [
        ops.payload,
        ops.message_read,
        ops.error,
        ops.success_payload,
        ops.success,
    ] {
        super::callbacks::effects(node, NodeCategory::Computation)?;
    }

    super::callbacks::effects(ops.selector, NodeCategory::Control)?;

    super::callbacks::effects(ops.merge, NodeCategory::Merge)?;
    read(ops.environment, &checked.message.symbol)?;
    read(
        ops.error_read,
        checked
            .binding
            .binding
            .as_deref()
            .ok_or("Missing error symbol")?,
    )?;

    super::verify_binding_initialization(ops.declaration, checked.binding)?;

    if ops.declaration.category != NodeCategory::Declaration {
        return Err("ID error callback loses owned parameter initialization".into());
    }

    Ok(())
}

fn read(node: &PdgNode, symbol: &str) -> Result<(), String> {
    if node.category != NodeCategory::Read
        || node.attributes.symbol_ids != [symbol]
        || !matches!(node.attributes.reads.as_slice(), [read]
            if read.symbol_id.as_deref() == Some(symbol) && read.location_id.is_none())
        || !node.attributes.defines.is_empty()
        || !node.attributes.writes.is_empty()
        || node.attributes.operator.is_some()
        || node.attributes.literal.is_some()
        || node.attributes.callee.is_some()
        || node.attributes.call_form.is_some()
        || node.attributes.write_form.is_some()
        || node.attributes.position.is_some()
        || node.attributes.exit_kind.is_some()
        || !node.attributes.outcomes.is_empty()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err(
            "ID error callback read changes exact owned binding or immutable capture".into(),
        );
    }

    Ok(())
}

fn operands(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for (node, values) in [
        (ops.selector, vec![ops.receiver]),
        (ops.payload, vec![ops.receiver]),
        (ops.declaration, vec![ops.payload]),
        (ops.message_read, vec![ops.environment]),
        (ops.constructor, vec![ops.error_read, ops.message_read]),
        (ops.error, vec![ops.constructor]),
        (ops.success_payload, vec![ops.receiver]),
        (ops.success, vec![ops.success_payload]),
        (ops.merge, vec![]),
    ] {
        super::verify_saved_operands(pair, node, &values)?;
    }

    for (value, consumers) in [
        (
            ops.receiver,
            vec![ops.selector, ops.payload, ops.success_payload],
        ),
        (ops.environment, vec![ops.message_read]),
        (ops.payload, vec![ops.declaration]),
        (ops.error_read, vec![ops.constructor]),
        (ops.message_read, vec![ops.constructor]),
        (ops.constructor, vec![ops.error]),
        (ops.success_payload, vec![ops.success]),
    ] {
        super::value_consumers::verify(pair, value, &consumers, "value_operand")?;
    }

    Ok(())
}

fn binding(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    checked: &contract::Checked<'_>,
    ops: &Operations<'_>,
) -> Result<(), String> {
    let formal = super::serialized_operation(pair, &checked.message.pat.span, |node| {
        node.category == NodeCategory::FormalInput
    })?;

    super::copy_tagged_values::origins::verify_definition(pair, formal, ops.environment)?;

    super::copy_tagged_values::origins::verify_definition(pair, ops.declaration, ops.error_read)?;

    super::value_consumers::verify(
        pair,
        ops.declaration,
        &[ops.error_read],
        "reaching_definition",
    )?;

    let checked_dependency = crate::helper_calls::verify_use(checked.constructor, facts)?;

    if checked_dependency.summary().identity != crate::helper_calls::named_errors::OWNED_SUMMARY {
        return Err("ID error callback loses its selected whole-owned constructor proof".into());
    }

    Ok(())
}
