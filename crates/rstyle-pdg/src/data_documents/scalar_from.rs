//! Independent concrete conversion inventory, effects and saved operand checking.
use super::*;
use crate::{HirNode, producer::PairView, scalar_from, unit_store::EvidenceFacts};

pub(super) fn verify(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    calls: &[&HirNode],
) -> Result<(), String> {
    let sources: Vec<_> = calls
        .iter()
        .copied()
        .filter(|node| scalar_from::candidate(node, facts))
        .collect();
    let actual = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some(scalar_from::CONSTRUCT))
        .count();
    if actual != sources.len() {
        return Err("Scalar From inventory disagrees with source".into());
    }
    if actual != 0
        && !crate::extraction_profile::policy_supports(&pair.pdg.profile, scalar_from::CAPABILITY)
    {
        return Err("Scalar From requires its registered document policy".into());
    }

    for source in sources {
        verify_call(pair, facts, source)?;
    }
    Ok(())
}

fn verify_call(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    source: &HirNode,
) -> Result<(), String> {
    let summary = scalar_from::resolve(source, facts)?.ok_or("Unverified scalar From")?;
    let node = super::serialized_operation(pair, &source.span, |node| {
        node.category == NodeCategory::Call
    })?;
    if node.attributes.construct.as_deref() != Some(scalar_from::CONSTRUCT)
        || node.original_kind.known_value() != Some("expr_call")
        || node
            .annotations
            .get("compat.rust.hir-r0:type")
            .and_then(|value| value.get("value"))
            .and_then(serde_json::Value::as_str)
            != Some("u32")
        || node.attributes.call_form != Some(crate::CallForm::Direct)
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || node.attributes.literal.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Scalar From loses its exact value semantics/effects".into());
    }
    super::summary_records::Record {
        source,
        node,
        summary,
    }
    .verify(pair, facts)
}
