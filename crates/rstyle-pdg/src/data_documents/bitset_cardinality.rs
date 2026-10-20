//! Independently check typed integer operations and their complete source inventory.
use crate::{
    HirNode, bitset_cardinality as integer, producer::PairView, unit_store::EvidenceFacts,
};

pub(super) fn verify(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    calls: &[&HirNode],
) -> Result<(), String> {
    let sources: Vec<_> = calls
        .iter()
        .copied()
        .filter(|node| integer::kind(node, facts).is_some())
        .collect();
    let actual = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.attributes.construct.as_deref().is_some_and(|name| {
                name.starts_with("data.bitset.") || name.starts_with("data.integer.")
            })
        })
        .count();

    if actual != sources.len() {
        return Err("Integer operation inventory disagrees with complete source".into());
    }
    if actual != 0
        && !crate::extraction_profile::policy_supports(&pair.pdg.profile, integer::CAPABILITY)
    {
        return Err("Integer operations require their registered document policy".into());
    }

    for source in sources {
        verify_call(pair, source, facts)?;
    }
    Ok(())
}

fn verify_call(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let kind = integer::kind(source, facts).ok_or("Missing integer operation")?;
    let summary = integer::resolve(source, facts)?.ok_or("Unverified integer operation")?;
    let node = super::serialized_operation(pair, &source.span, |node| {
        node.category == crate::NodeCategory::Call
    })?;

    if node.attributes.construct.as_deref() != Some(kind.construct())
        || node.original_kind.known_value() != Some(source.kind.as_str())
        || node
            .annotations
            .get("compat.rust.hir-r0:type")
            .and_then(|value| value.get("value"))
            .and_then(serde_json::Value::as_str)
            != Some(summary.result_type.as_str())
        || node.attributes.call_form != Some(crate::CallForm::Direct)
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || node.attributes.literal.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Integer operation loses exact type, value semantics or effects".into());
    }
    super::summary_records::Record {
        source,
        node,
        summary,
    }
    .verify(pair, facts)
}
