//! Exact fresh-error and eager conversion inventories, effects and saved order.
use super::*;
use crate::HirNode;
use crate::{borrowed_result_errors as errors, unit_store::EvidenceFacts};

pub(super) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<(), String> {
    for source in errors::errors(facts) {
        let checked = errors::verify_error(source, facts)?;
        let node = super::serialized_operation_mut(pair, source)?;
        if node.attributes.construct.as_deref() != Some(errors::ERROR) {
            return Err("Fresh error source lacks its exact operation".into());
        }
        node.annotations
            .insert(errors::KEY.into(), errors::error_semantics(&checked));
    }
    for source in errors::sources(facts) {
        errors::verify(source, facts)?;
        let node = super::serialized_operation_mut(pair, source)?;
        if node.attributes.construct.as_deref() != Some(errors::CONSTRUCT) {
            return Err("Eager conversion source lacks its exact call".into());
        }
        node.annotations
            .insert(errors::KEY.into(), errors::semantics());
    }

    Ok(())
}

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let values = errors::errors(facts);
    let calls = errors::sources(facts);
    super::verify_construct_inventory(
        pair,
        [errors::ERROR, errors::CONSTRUCT],
        [values.len(), calls.len()],
    )?;
    let annotated = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.annotations.contains_key(errors::KEY))
        .count();
    if annotated != values.len() + calls.len() {
        return Err("Fresh error semantic inventory disagrees with original source".into());
    }
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        errors::CAPABILITY,
        || !values.is_empty() || !calls.is_empty(),
        "Borrowed Result operations require their registered document policy",
    )?;
    for source in values {
        verify_error(pair, source, facts)?;
    }
    for source in calls {
        verify_call(pair, source, facts)?;
    }

    Ok(())
}

fn verify_error(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let checked = errors::verify_error(source, facts)?;
    let node = super::typed_operation(
        pair,
        source,
        errors::ERROR,
        crate::data_aggregates::token(source)?,
    )?;

    if node.category != NodeCategory::Computation
        || node.annotations.get(errors::KEY) != Some(&errors::error_semantics(&checked))
        || node
            .attributes
            .literal
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some(checked.variant.as_str())
        || node.attributes.write_form.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Fresh error loses its exact source-proved unit variant".into());
    }
    super::verify_value_effects(node, true, "Fresh error invents effects")?;
    super::verify_saved_operands(pair, node, &[])
}

fn verify_call(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let checked = errors::verify(source, facts)?;
    let node = super::typed_operation(
        pair,
        source,
        errors::CONSTRUCT,
        crate::data_aggregates::token(source)?,
    )?;

    if node.category != NodeCategory::Call
        || node.annotations.get(errors::KEY) != Some(&errors::semantics())
    {
        return Err("Eager conversion loses exact saved-payload/error semantics".into());
    }
    super::verify_saved_operands(
        pair,
        node,
        &[
            super::value_operation(pair, checked.receiver)?,
            super::value_operation(pair, checked.error)?,
        ],
    )?;
    super::callbacks::order::operands(pair, checked.receiver, checked.error, node)?;
    super::summary_records::Record {
        source,
        node,
        summary: errors::resolve(source, facts)?,
    }
    .verify_pure(pair, facts, errors::CONSTRUCT)
}
