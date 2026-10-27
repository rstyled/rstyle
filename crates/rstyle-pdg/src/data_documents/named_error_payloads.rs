//! Complete fresh errors and checked arithmetic callers, independently replayed.
mod graph;
mod origins;
mod source;

use super::*;
use crate::{HirNode, unit_store::EvidenceFacts};

pub(crate) const KEY: &str = "rstyle.data:named-error-value-origins";

pub(crate) fn verify_source(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    crate::source_evidence::source_present(facts, source::verify)
}

pub(crate) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let installed = super::guarded_sources::Policy {
        key: KEY,
        coverage: super::guarded_sources::Coverage::Matching,
    }
    .install(
        pair,
        facts,
        super::guarded_sources::Replay {
            source: source::verify,
            graph: graph::verify,
        },
    )?;
    graph::install_owned(pair, facts)?;
    Ok(installed)
}

pub(crate) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    graph::verify_owned(pair, facts)?;

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
