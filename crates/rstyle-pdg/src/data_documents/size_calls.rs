//! Independent scalar-size inventory and semantic interpretation.
use super::*;
use crate::{producer::PairView, size_calls, unit_store::EvidenceFacts};

pub(super) fn sources(facts: &dyn EvidenceFacts) -> Vec<&crate::HirNode> {
    crate::field_locations::selected_operations(facts, &["expr_call"], size_calls::candidate)
}

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let sources = sources(facts);
    let actual = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some(size_calls::CONSTRUCT))
        .count();
    if actual != sources.len()
        || pair.pdg.nodes.iter().any(|node| {
            node.annotations.contains_key(size_calls::KEY)
                && node.attributes.construct.as_deref() != Some(size_calls::CONSTRUCT)
        })
    {
        return Err("Scalar size operation inventory disagrees".into());
    }
    if actual != 0
        && !crate::extraction_profile::policy_supports(&pair.pdg.profile, size_calls::CAPABILITY)
    {
        return Err("Scalar size requires its registered document policy".into());
    }

    for source in sources {
        verify_call(pair, facts, source)?;
    }
    Ok(())
}

fn verify_call(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    source: &crate::HirNode,
) -> Result<(), String> {
    let node = super::serialized_operation(pair, &source.span, |node| {
        node.category == NodeCategory::Call
    })?;
    if node.attributes.construct.as_deref() != Some(size_calls::CONSTRUCT)
        || node.original_kind.known_value() != Some("expr_call")
        || node.attributes.call_form != Some(crate::CallForm::Direct)
        || node.annotations.get(size_calls::KEY) != Some(&size_calls::semantics(source, facts)?)
        || node
            .annotations
            .get("compat.rust.hir-r0:type")
            .and_then(|v| v.get("value"))
            .and_then(serde_json::Value::as_str)
            != Some("usize")
        || node.attributes.literal.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Scalar size loses its exact call/value interpretation".into());
    }
    if !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
    {
        return Err("Scalar size invents memory or binding effects".into());
    }
    super::summary_records::Record {
        source,
        node,
        summary: size_calls::resolve(source, facts)?.ok_or("Missing scalar size summary")?,
    }
    .verify(pair, facts)
}
