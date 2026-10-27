//! Source-derived termination for one bounded scalar factor-counting family.
//!
//! This is a normal-flow helper boundary proof, not a complete panic CFG.
mod graph;
mod source;

use crate::{producer::PairView, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.bounded-scalar-loop-helpers";
pub(crate) const SUMMARY: &str = "rstyle.helper.bounded-factor-loop/1";

/// Properties independently derived from exact source and checked cyclic flow.
pub(crate) struct Checked {
    pub may_panic: bool,
    pub may_diverge: bool,
    pub positive_max_backedges: u32,
}

pub(crate) fn verify(facts: &dyn EvidenceFacts, pair: PairView<'_>) -> Result<Checked, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || crate::data_library::registry_for(facts.facts()).is_none()
        || facts.build.debug_assertions.status != crate::FactStatus::Known
        || facts.build.debug_assertions.value != Some(true)
        || facts.build.overflow_checks.status != crate::FactStatus::Known
        || facts.build.overflow_checks.value != Some(true)
        || facts.build.panic_strategy.known_value() != Some("unwind")
    {
        return Err("Bounded scalar helper requires its pinned checked debug context".into());
    }
    let shape = source::verify(facts)?;
    graph::verify(facts, pair, &shape)?;
    // Nonzero divisible values shrink by five; 5^13 <= u32::MAX < 5^14.
    // Zero reaches the first assertion before either division or count update.
    Ok(Checked {
        may_panic: true,
        may_diverge: false,
        positive_max_backedges: 13,
    })
}

/// Every cyclic helper must independently satisfy the complete bounded proof.
pub(crate) fn verify_if_loop(
    facts: &dyn EvidenceFacts,
    pair: PairView<'_>,
) -> Result<Option<Checked>, String> {
    if !has_loop(pair) {
        return Ok(None);
    }
    verify(facts, pair).map(Some)
}

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    if crate::extraction_profile::policy_supports(profile, CAPABILITY) {
        return Ok(());
    }
    for call in crate::helper_calls::call_nodes(facts) {
        if crate::helper_calls::verify_use(call, facts)?
            .summary()
            .bounded_loop
            .is_some()
        {
            return Err(
                "Bounded scalar looping helpers require their registered document policy".into(),
            );
        }
    }
    Ok(())
}

pub(crate) fn has_loop(pair: PairView<'_>) -> bool {
    pair.pdg
        .nodes
        .iter()
        .any(|node| matches!(node.attributes.construct.as_deref(), Some("loop" | "while")))
}
