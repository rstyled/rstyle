//! Candidate linking and exact selected-member authority remain separate.
use crate::{HirNode, unit_store::EvidenceFacts};
use serde_json::Value;

pub(crate) const CAPABILITY: &str = "rust.copy-tagged-conversion-helper-calls";

pub(crate) fn selected_resolution(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<Value> {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        .then(|| crate::copy_conversion_members::calls::selected_resolution(node, facts))?
}

pub(crate) fn verify(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
) -> Result<bool, String> {
    if selected_resolution(node, caller).is_none() {
        return Ok(false);
    }
    if !crate::extraction_profile::facts_support(callee, CAPABILITY) {
        return Err(
            "Tagged conversion dependency requires its registered helper capability".into(),
        );
    }
    crate::copy_conversion_members::calls::verify(node, caller, callee)?;
    Ok(true)
}
