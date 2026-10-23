//! Owning checked tokens activate only the proved original byte constructor.
//! Raw generic facts, diagnostic observations and matching names grant nothing.
use super::{CheckedProjection, Haystack};
use crate::unit_store::EvidenceFacts;
use crate::{HirFnFacts, HirNode, data_origins::Origin, field_locations::Projection};

#[cfg(test)]
mod tests;

pub const CAPABILITY: &str = "rust.checked-reference-constructor-instances";
pub const CONSTRUCT: &str = "data.reference-instance.as-ref";

/// Absence stays ordinary source evidence. A str-to-bytes view needs its own
/// origin projection before it can enter this type-preserving activation path.
pub fn verify(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = facts.reference_instance() else {
        return Ok(false);
    };
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Checked reference instance requires its registered capability".into());
    }
    owning(checked, facts.facts())?;
    Ok(true)
}

fn owning(checked: &CheckedProjection, facts: &HirFnFacts) -> Result<(), String> {
    if checked.facts() != facts {
        return Err("Checked reference instance differs from its owning facts".into());
    }
    if checked.haystack() != Haystack::Bytes {
        return Err(
            "Str constructor activation requires an explicit byte-view origin proof".into(),
        );
    }
    findings(facts, checked.as_ref())
}

/// Coverage of the exact preserved finding, never deletion of raw diagnostics.
pub fn source_findings(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    verify(facts)
}

fn findings(facts: &HirFnFacts, call: &HirNode) -> Result<(), String> {
    let [finding] = facts.source_diagnostics.as_slice() else {
        return Err("Checked constructor requires its exact deferred AsRef finding roster".into());
    };
    if facts.source_scan_complete
        || finding.capability != "rust.call-summaries"
        || finding.construct != "expr_method_call"
        || finding.span != call.span
        || finding.snippet != call.snippet
        || finding.severity != "error"
        || finding.reason
            != "Original owned source contains deferred rust.call-summaries; cfg removal cannot certify a supported remainder."
    {
        return Err(
            "Checked constructor deferred AsRef finding differs from its proved source site".into(),
        );
    }
    Ok(())
}

/// One checked, type-preserving conversion; evaluates operand zero exactly once.
pub struct Operation<'a> {
    pub receiver: &'a HirNode,
    pub origin: Origin,
}

pub fn operation<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Option<Operation<'a>>, String> {
    if !verify(facts)? {
        return Ok(None);
    }
    let checked = facts
        .reference_instance()
        .ok_or("Missing checked reference instance after activation")?;
    checked_operation(node, checked.as_ref())
}

fn checked_operation<'a>(
    node: &'a HirNode,
    expected: &HirNode,
) -> Result<Option<Operation<'a>>, String> {
    if node.hir != expected.hir {
        return Ok(None);
    }
    if node != expected {
        return Err("Checked AsRef operation differs from its exact original HIR/source".into());
    }
    let [receiver] = node.children.as_slice() else {
        return Err("Checked AsRef operation requires exactly operand zero".into());
    };
    Ok(Some(Operation {
        receiver,
        origin: Origin {
            parameter: 0,
            path: vec![Projection::SharedReferent],
        },
    }))
}
