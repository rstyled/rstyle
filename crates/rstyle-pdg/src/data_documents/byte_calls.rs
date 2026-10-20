//! Independently validate each typed byte call, exact operand and memory effects.

use super::*;
use crate::{data_bytes, producer::PairView};

pub(super) fn sources(facts: &dyn crate::unit_store::EvidenceFacts) -> Vec<&crate::HirNode> {
    if !data_bytes::enabled(facts) {
        return Vec::new();
    }
    crate::field_locations::operations(&facts.body, &["expr_call", "expr_method_call"])
        .into_iter()
        .filter(|node| data_bytes::kind(node).is_some())
        .collect()
}

pub(super) fn install(
    pair: &mut Pair,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    super::read_accesses::install(pair, context, super::read_accesses::Kind::Byte)
}

pub(super) fn verify(
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
    calls: &[&crate::HirNode],
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    let enabled = data_bytes::enabled(context.facts);
    let sources: Vec<_> = calls
        .iter()
        .copied()
        .filter(|node| enabled && data_bytes::kind(node).is_some())
        .collect();
    let actual = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.annotations.contains_key(data_bytes::KEY))
        .count();
    if actual != sources.len() {
        return Err("Byte-call semantic inventory disagrees".into());
    }
    let mut locations = BTreeMap::new();
    for source in sources {
        let kind = data_bytes::verify(source, context.facts)?;
        let node = super::serialized_operation(pair, &source.span, |node| {
            node.category == NodeCategory::Call
        })?;
        let (reads, entries) =
            context.read_accesses(data_bytes::locations(source, context.facts, kind)?)?;
        verify_node(node, kind, &reads)?;
        verify_record(pair, source, node, context.facts)?;
        locations.extend(entries);
    }

    Ok(locations)
}

fn verify_node(
    node: &crate::PdgNode,
    kind: data_bytes::Kind,
    reads: &[crate::Access],
) -> Result<(), String> {
    if node.attributes.construct.as_deref() != Some(kind.construct())
        || node.annotations.get(data_bytes::KEY) != Some(&kind.semantics())
        || node.attributes.reads != reads
        || !node.attributes.writes.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || !node.attributes.defines.is_empty()
        || node.attributes.literal.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
        || node.attributes.call_form != Some(crate::CallForm::Direct)
    {
        return Err("Byte call loses its exact semantics or effects".into());
    }
    Ok(())
}

fn verify_record(
    pair: PairView<'_>,
    source: &crate::HirNode,
    node: &crate::PdgNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    let summary = data_bytes::resolve(source, facts)?.ok_or("Unverified byte call")?;
    let records = pair
        .pdg
        .extensions
        .get(crate::call_summaries::KEY)
        .and_then(serde_json::Value::as_array)
        .ok_or("Missing call summary inventory")?;
    let matched: Vec<_> = records
        .iter()
        .filter(|record| record["pdg_node_id"] == node.id)
        .collect();
    let [record] = matched.as_slice() else {
        return Err("Missing or duplicate byte summary".into());
    };

    crate::call_summaries::verify_build(record, facts)?;
    if record["summary"] != serde_json::to_value(&summary).map_err(|error| error.to_string())?
        || record.get("dependency").is_some()
        || record.get("library").is_some()
        || node
            .attributes
            .callee
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some(&summary.callee_path)
        || node
            .attributes
            .operator
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some(&summary.callee_path)
        || record["source_span"]
            != serde_json::to_value(source.span).map_err(|error| error.to_string())?
    {
        return Err("Byte-call summary disagrees with concrete authority".into());
    }
    crate::helper_calls::verify_call_operands(pair, node, source, record)
}
