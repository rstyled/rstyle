//! Whole fallible native reader composition; no partial source or origin authority.
mod graph;
mod native;
mod origins;
mod source;
#[cfg(test)]
mod tests;

use super::*;
use crate::{HirNode, named_error_result_propagation as propagation, unit_store::EvidenceFacts};

pub(crate) const KEY: &str = "rstyle.data:fallible-reader-origins";

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, propagation::CAPABILITY)
        && facts.body.children.first().is_some_and(|statement| {
            statement.kind == "stmt_semi"
                && statement
                    .children
                    .first()
                    .is_some_and(|node| propagation::candidate(node, facts))
        })
}

pub(crate) fn verify_source(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    crate::source_evidence::source_present(facts, source::verify)
}

pub(crate) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    super::guarded_sources::Policy {
        key: KEY,
        coverage: super::guarded_sources::Coverage::Complete(
            "Unaccounted fallible reader operation",
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
            return Err("Unproved function claims complete fallible reader origins".into());
        }
        return Ok(false);
    };
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        propagation::CAPABILITY,
        || true,
        "Fallible native reader requires its registered document policy",
    )?;

    let expected = graph::verify(pair, facts, &checked)?;
    for node in &pair.pdg.nodes {
        if node.annotations.get(KEY) != expected.get(&node.id)
            || node.annotations.contains_key(crate::data_origins::KEY)
        {
            return Err(
                "Fallible reader origins differ from exact guarded source/graph roles".into(),
            );
        }
    }

    if pair.pdg.extensions.contains_key(KEY) {
        return Err(
            "Fallible reader origins cannot replace node-specific proof with a global claim".into(),
        );
    }
    Ok(true)
}
