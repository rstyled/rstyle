//! Complete ID reader compositions preserve checked errors, reads and panic.
mod graph;
mod origins;
mod source;

use super::*;
use crate::{HirNode, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.named-error-id-reader-flow";
pub(crate) const KEY: &str = "rstyle.data:id-reader-origins";

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    source::candidate(facts)
}

pub(crate) fn source_contract(
    facts: &dyn EvidenceFacts,
) -> Result<crate::copy_tagged_values::origins::WireContract, String> {
    let checked = source::verify(facts)?.ok_or("Missing complete ID reader source")?;
    origins::contract(facts, &checked)
}

pub(crate) fn verify_source(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    crate::source_evidence::source_present(facts, source::verify)
}

pub(crate) fn source_findings(
    facts: &dyn EvidenceFacts,
) -> Result<Option<Vec<crate::HirDiagnostic>>, String> {
    let Some(checked) = source::verify(facts)? else {
        return Ok(None);
    };
    if !facts.source_diagnostics.is_empty() {
        crate::source_evidence::checked_activation::exact_deferred_call(
            facts.facts(),
            checked.map.source,
        )?;
    }
    Ok(Some(Vec::new()))
}

pub(crate) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    super::guarded_sources::Policy {
        key: KEY,
        coverage: super::guarded_sources::Coverage::Complete("Unaccounted ID reader operation"),
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
            return Err("Unproved source claims complete ID reader origins".into());
        }
        return Ok(false);
    };
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        CAPABILITY,
        || true,
        "ID readers require their registered document policy",
    )?;
    let expected = graph::verify(pair, facts, &checked)?;
    for node in &pair.pdg.nodes {
        if node.annotations.get(KEY) != expected.get(&node.id)
            || node.annotations.contains_key(crate::data_origins::KEY)
        {
            return Err("ID reader origins differ from complete source and graph roles".into());
        }
    }
    if pair.pdg.extensions.contains_key(KEY) {
        return Err("ID reader origins require per-operation evidence".into());
    }

    Ok(true)
}
