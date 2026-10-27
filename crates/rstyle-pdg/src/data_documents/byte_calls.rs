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
        verify_policy(pair, source, kind)?;
        let node = super::serialized_operation(pair, &source.span, |node| {
            node.category == NodeCategory::Call
        })?;
        let (reads, entries) =
            context.read_accesses(data_bytes::locations(source, context.facts, kind)?)?;
        verify_node(node, kind, &reads)?;
        super::summary_records::Record {
            source,
            node,
            summary: data_bytes::resolve(source, context.facts)?.ok_or("Unverified byte call")?,
        }
        .verify(pair, context.facts)?;
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

fn verify_policy(
    pair: PairView<'_>,
    source: &crate::HirNode,
    kind: data_bytes::Kind,
) -> Result<(), String> {
    if kind.requires_checked_flow()
        && !crate::extraction_profile::policy_supports(
            &pair.pdg.profile,
            data_bytes::CHECKED_FLOW_CAPABILITY,
        )
    {
        return Err("Checked borrowed bytes require their registered document policy".into());
    }
    if kind.requires_prefix(source)
        && !crate::extraction_profile::policy_supports(
            &pair.pdg.profile,
            crate::data_subslices::PREFIX_CAPABILITY,
        )
    {
        return Err(
            "Extended byte signatures/displays require their registered document policy".into(),
        );
    }
    if matches!(kind, data_bytes::Kind::Copy(_))
        && source
            .children
            .first()
            .and_then(|receiver| receiver.children.first())
            .is_some_and(|base| {
                matches!(
                    crate::data_aggregates::parsed_type(base),
                    Ok(crate::data_types::TypeToken::Array { .. })
                )
            })
        && !crate::extraction_profile::policy_supports(
            &pair.pdg.profile,
            "rust.accelerator-packing",
        )
    {
        return Err("Implicit array-place copies require their registered document policy".into());
    }
    Ok(())
}
