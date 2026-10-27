//! Complete fallible reader products, preserving static error roots and effects.
mod graph;
mod origins;
mod source;

use super::*;
use crate::{HirNode, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.named-error-product-reader-flow";
pub(crate) const KEY: &str = "rstyle.data:reader-product-origins";

pub(crate) fn verify_source(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    crate::source_evidence::source_present(facts, source::verify)
}

pub(crate) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    super::guarded_sources::Policy {
        key: KEY,
        coverage: super::guarded_sources::Coverage::Complete(
            "Unaccounted reader product operation",
        ),
    }
    .install(
        pair,
        facts,
        super::guarded_sources::Replay {
            source: source::verify,
            graph: graph::verify,
        },
    )
}

pub(crate) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = source::verify(facts)? else {
        if pair
            .pdg
            .nodes
            .iter()
            .any(|node| node.annotations.contains_key(KEY))
            || pair.pdg.extensions.contains_key(KEY)
        {
            return Err("Unproved function claims complete reader product origins".into());
        }
        return Ok(false);
    };
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        CAPABILITY,
        || true,
        "Reader products require their registered document policy",
    )?;

    let expected = graph::verify(pair, facts, &checked)?;
    for node in &pair.pdg.nodes {
        if node.annotations.get(KEY) != expected.get(&node.id)
            || node.annotations.contains_key(crate::data_origins::KEY)
        {
            return Err(
                "Reader product origins differ from complete source and graph roles".into(),
            );
        }
    }
    if pair.pdg.extensions.contains_key(KEY) {
        return Err("Reader product origins require per-operation evidence".into());
    }
    Ok(true)
}
