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

    super::summary_records::Record {
        source,
        node,
        summary,
    }
    .verify_pure(pair, facts, kind.construct())
}
