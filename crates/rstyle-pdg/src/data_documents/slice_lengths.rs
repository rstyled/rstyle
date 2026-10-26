//! Independently replay complete length guards and both original Result returns.
mod graph;
mod origins;
mod source;

use super::*;
use crate::{copy_tagged_values::origins::WireContract, unit_store::EvidenceFacts};

pub(crate) const KEY: &str = "rstyle.data:checked-slice-length-origins";

pub(crate) fn source_contract(facts: &dyn EvidenceFacts) -> Result<WireContract, String> {
    crate::source_evidence::source_contract(facts, source::verify, origins::contract)
}

pub(crate) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if !crate::helper_calls::slice_lengths::candidate(facts) {
        return Ok(false);
    }
    let checked = source::verify(facts)?;
    let expected = graph::verify(pair.as_view(), &checked)?;

    for node in &mut pair.pdg.nodes {
        node.annotations.insert(
            KEY.into(),
            expected
                .get(&node.id)
                .ok_or("Unaccounted slice-length operation")?
                .clone(),
        );
    }
    Ok(true)
}

pub(crate) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !crate::helper_calls::slice_lengths::candidate(facts) {
        if pair
            .pdg
            .nodes
            .iter()
            .any(|node| node.annotations.contains_key(KEY))
        {
            return Err("Unproved function claims checked slice-length origins".into());
        }
        return Ok(());
    }
    verified_contract(pair, facts).map(|_| ())
}

/// Reuse one complete source replay for graph validation and summary projection.
pub(crate) fn verified_contract(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<WireContract, String> {
    if !crate::helper_calls::slice_lengths::candidate(facts) {
        return Err("Checked slice-length contract requires its selected source family".into());
    }
    let checked = source::verify(facts)?;
    let expected = graph::verify(pair, &checked)?;
    for node in &pair.pdg.nodes {
        if node.annotations.get(KEY) != expected.get(&node.id)
            || node.annotations.contains_key(crate::data_origins::KEY)
        {
            return Err("Slice-length origins change exact guarded source roles".into());
        }
    }

    verify_document_policy(&pair.pdg.profile, facts)?;
    origins::contract(&checked)
}

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        crate::helper_calls::slice_lengths::CAPABILITY,
        || crate::helper_calls::slice_lengths::candidate(facts),
        "Checked slice-length helper requires its registered document policy",
    )
}
