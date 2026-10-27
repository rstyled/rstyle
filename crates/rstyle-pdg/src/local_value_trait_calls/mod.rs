//! Bounded primitive by-value local trait association, never helper effects.
//!
//! Integration must gate this leaf with its registered capability, resolve the
//! original trait path lexically, and retain whole-namespace source checks. The
//! callee's ordinary checked body/summary remains the only effects authority.
mod contract;
mod model;
mod selection;
mod source;
pub(crate) const CAPABILITY: &str = "rust.local-value-trait-calls";
pub(crate) const U32_USIZE_CAPABILITY: &str = "rust.u32-usize-value-trait-calls";
pub(crate) use contract::{Contract, captured as captured_contract};
#[cfg(test)]
mod tests;

pub(crate) use selection::{association, verify as verify_selection};
pub(crate) use source::{SourceMember, member as source_member, verify_trait};

#[derive(serde::Serialize)]
struct Resolution {
    local: bool,
    #[serde(flatten)]
    association: selection::Association,
}

pub(crate) fn verify_document_policy(
    pair: crate::producer::PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
    calls: &[&crate::HirNode],
) -> Result<(), String> {
    for call in calls {
        if let Some(contract) = contract::dispatch(call)
            && !crate::extraction_profile::policy_supports(&pair.pdg.profile, contract.capability())
            && resolution(call, facts).is_some()
        {
            return Err("Local value trait call requires its registered document policy".into());
        }
    }
    Ok(())
}

pub(crate) fn resolution(
    node: &crate::HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Option<serde_json::Value> {
    let dispatch = node.dispatch.as_ref()?;
    let contract = contract::dispatch(node)?;
    if node.kind != "expr_method_call"
        || dispatch.kind != "resolved_method"
        || !crate::extraction_profile::facts_support(facts, contract.capability())
    {
        return None;
    }

    let selected = association(node, facts).ok()?;

    serde_json::to_value(Resolution {
        local: true,
        association: selected,
    })
    .ok()
}
