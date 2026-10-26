//! Complete fresh errors and checked arithmetic callers, independently replayed.
mod graph;
mod origins;
mod source;

use super::*;
use crate::{HirNode, unit_store::EvidenceFacts};

pub(crate) const KEY: &str = "rstyle.data:named-error-value-origins";

pub(crate) fn verify_source(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    source::verify(facts).map(|checked| checked.is_some())
}

pub(crate) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = source::verify(facts)? else {
        return Ok(false);
    };
    let expected = graph::verify(pair.as_view(), facts, &checked)?;
    for node in &mut pair.pdg.nodes {
        if let Some(origin) = expected.get(&node.id) {
            node.annotations.insert(KEY.into(), origin.clone());
        }
    }
    Ok(true)
}

pub(crate) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let Some(checked) = source::verify(facts)? else {
        if pair
            .pdg
            .nodes
            .iter()
            .any(|node| node.annotations.contains_key(KEY))
        {
            return Err("Unproved function claims fresh named error origins".into());
        }
        return Ok(());
    };
    let expected = graph::verify(pair, facts, &checked)?;
    for node in &pair.pdg.nodes {
        if node.annotations.get(KEY) != expected.get(&node.id)
            || node.annotations.contains_key(crate::data_origins::KEY)
        {
            return Err("Named error origins differ from complete source and graph roles".into());
        }
    }
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        crate::named_error_payloads::CAPABILITY,
        || true,
        "Fresh named error requires its registered document policy",
    )
}
