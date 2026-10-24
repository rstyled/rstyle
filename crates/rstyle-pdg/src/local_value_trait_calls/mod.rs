//! Bounded primitive by-value local trait association, never helper effects.
//!
//! Integration must gate this leaf with its registered capability, resolve the
//! original trait path lexically, and retain whole-namespace source checks. The
//! callee's ordinary checked body/summary remains the only effects authority.
mod model;
mod selection;
mod source;
pub(crate) const CAPABILITY: &str = "rust.local-value-trait-calls";
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
    if !crate::extraction_profile::policy_supports(&pair.pdg.profile, CAPABILITY)
        && calls.iter().any(|call| resolution(call, facts).is_some())
    {
        return Err("Local value trait call requires its registered document policy".into());
    }
    Ok(())
}

pub(crate) fn resolution(
    node: &crate::HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Option<serde_json::Value> {
    let dispatch = node.dispatch.as_ref()?;
    if node.kind != "expr_method_call"
        || dispatch.kind != "resolved_method"
        || dispatch.operand_tys != ["usize"]
        || dispatch.result_ty != "u64"
        || !crate::extraction_profile::facts_support(facts, CAPABILITY)
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
