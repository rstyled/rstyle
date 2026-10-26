//! One private original From proof selects its checked constructor dependency.
use crate::{HirNode, unit_store::EvidenceFacts};
use serde_json::Value;

#[cfg(test)]
mod tests;

fn selected<'a>(
    node: &HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Option<&'a crate::reference_instances::CheckedForwarderProjection> {
    let checked = facts.reference_forwarder()?;
    (crate::reference_instances::forwarder_activation::verify(facts).ok()?
        && node == checked.call())
    .then_some(checked)
}

pub(super) fn resolution(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<Value> {
    let checked = selected(node, facts)?;
    super::local_resolution(checked.constructor().facts())
}

pub(super) fn callee(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<String> {
    Some(selected(node, facts)?.constructor().facts().name.clone())
}

pub(super) fn verify(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
) -> Result<bool, String> {
    let Some(checked) = selected(node, caller) else {
        return Ok(false);
    };
    let expected = checked.constructor();
    if callee.facts() != expected.facts()
        || !callee.reference_instance().is_some_and(|actual| {
            actual.facts() == expected.facts()
                && actual.haystack() == checked.constructor().haystack()
        })
        || !crate::reference_instances::activation::verify(callee)?
    {
        return Err("Checked From call selects another constructor owner or substitution".into());
    }
    Ok(true)
}
