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
        .filter(|node| {
            integer::kind(node, facts).is_some()
                || crate::lossless_integer_calls::kind(node, facts).is_some()
                || crate::widening_readers::scoped_conversion_candidate(node, facts)
        })
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
    verify_message_inventory(pair, facts, &sources)?;
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

fn verify_message_inventory(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    sources: &[&HirNode],
) -> Result<(), String> {
    let messages = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.attributes.construct.as_deref()
                == Some(crate::lossless_integer_calls::message::CONSTRUCT)
        })
        .count();
    if messages
        != sources
            .iter()
            .filter(|source| {
                crate::lossless_integer_calls::kind(source, facts)
                    .is_some_and(|kind| !kind.is_conversion())
            })
            .count()
    {
        return Err("Total expect literal inventory disagrees with complete source".into());
    }
    Ok(())
}

fn verify_call(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    if crate::widening_readers::scoped_conversion_candidate(source, facts) {
        return verify_scoped_u16(pair, source, facts);
    }
    if let Some(kind) = crate::lossless_integer_calls::kind(source, facts) {
        if !crate::extraction_profile::policy_supports(&pair.pdg.profile, kind.capability()) {
            return Err("Lossless conversion requires its registered document policy".into());
        }
        let summary = crate::call_summaries::resolve_for(source, facts)?
            .ok_or("Unverified lossless conversion")?;
        let node = super::serialized_operation(pair, &source.span, |node| {
            node.category == crate::NodeCategory::Call
        })?;
        super::summary_records::Record {
            source,
            node,
            summary,
        }
        .verify_pure(pair, facts, kind.construct())?;
        return if !kind.is_conversion() {
            verify_message(pair, source, facts)
        } else {
            Ok(())
        };
    }
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

fn verify_message(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let message = source
        .children
        .get(1)
        .ok_or("Missing total expect diagnostic operand")?;

    if !crate::lossless_integer_calls::message::verify(message, facts)? {
        return Err("Unverified total expect static literal".into());
    }
    let node = super::serialized_operation(pair, &message.span, |node| {
        node.category == crate::NodeCategory::Computation
    })?;
    if node.attributes.construct.as_deref()
        != Some(crate::lossless_integer_calls::message::CONSTRUCT)
        || node
            .attributes
            .literal
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != message.lit.as_deref()
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
    {
        return Err("Total expect loses its evaluated literal diagnostic value".into());
    }
    Ok(())
}

fn verify_scoped_u16(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        crate::widening_readers::U16_CAPABILITY,
        || true,
        "Scoped u16 conversion requires its complete reader document policy",
    )?;
    let summary = crate::widening_readers::conversion_summary(source, facts)?
        .ok_or("Missing whole-reader scoped u16 conversion authority")?;
    let node = super::serialized_operation(pair, &source.span, |node| {
        node.category == crate::NodeCategory::Call
    })?;
    super::summary_records::Record {
        source,
        node,
        summary,
    }
    .verify_pure(pair, facts, "data.integer.lossless-try-u16-usize")
}
