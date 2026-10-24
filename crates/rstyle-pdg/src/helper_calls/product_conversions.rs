//! Checked Into delegation selects an exact original local From dependency.
use crate::{HirNode, product_instances::SpanArgument, unit_store::EvidenceFacts};
use serde_json::{Value, json};

fn selected<'a>(
    node: &HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Option<&'a crate::product_instances::CheckedProjection> {
    let checked = facts.product_instance()?;
    (checked.argument() == SpanArgument::RangeUsize
        && crate::product_instances::activation::verify(facts).ok()?
        && checked.operation() == node)
        .then_some(checked)
}

pub(super) fn resolution(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<Value> {
    let callee = selected(node, facts)?.local_from()?;
    Some(json!({
        "local": true,
        "path": callee.name,
        "definition_identity": super::identity(callee)?,
    }))
}

pub(super) fn verify(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
) -> Result<bool, String> {
    let Some(checked) = selected(node, caller) else {
        return Ok(false);
    };

    let expected = checked
        .local_from()
        .ok_or("Missing checked local Range conversion")?;
    if callee.facts() != expected {
        return Err("Checked Into delegates to a different original From body/context".into());
    }

    crate::range_values::consuming::verify_fields(callee)?;
    Ok(true)
}
