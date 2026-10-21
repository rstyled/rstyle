//! Checked arithmetic retains exact source operands and nonpanicking overflow.
use super::*;
use crate::{HirNode, checked_lengths as lengths, unit_store::EvidenceFacts};

pub(super) fn sources(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::field_locations::selected_operations(facts, &["expr_method_call"], |node, facts| {
        lengths::kind(node, facts).is_some()
    })
}

pub(super) fn semantics(
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<serde_json::Value, String> {
    let kind = lengths::kind(source, facts).ok_or("Missing checked length operation")?;
    lengths::verify(source, kind, facts)?;
    Ok(lengths::semantics(kind))
}

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let sources = sources(facts);
    let kinds = [lengths::Kind::Add, lengths::Kind::Mul];

    super::verify_construct_inventory(
        pair,
        kinds.map(|kind| kind.construct()),
        kinds.map(|kind| {
            sources
                .iter()
                .filter(|source| lengths::kind(source, facts) == Some(kind))
                .count()
        }),
    )?;
    if !sources.is_empty()
        && !crate::extraction_profile::policy_supports(&pair.pdg.profile, lengths::CAPABILITY)
    {
        return Err("Checked length operations require their serialized document policy".into());
    }
    if pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.annotations.contains_key(lengths::KEY))
        .count()
        != sources.len()
    {
        return Err("Checked length interpretation inventory disagrees with source".into());
    }
    for source in sources {
        let kind = lengths::kind(source, facts).ok_or("Missing checked length kind")?;
        let summary = lengths::resolve(source, facts)?.ok_or("Unverified checked length")?;
        let node = super::typed_operation(pair, source, kind.construct(), &summary.result_type)?;
        if node.annotations.get(lengths::KEY) != Some(&lengths::semantics(kind)) {
            return Err("Checked length loses exact overflow or evaluation-order semantics".into());
        }
        super::summary_records::Record {
            source,
            node,
            summary,
        }
        .verify_pure(pair, facts, kind.construct())?;
    }
    Ok(())
}
