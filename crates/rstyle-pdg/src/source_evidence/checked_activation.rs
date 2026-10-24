//! Shared checks for private instance activation and preserved source findings.
use crate::{HirFnFacts, HirNode, unit_store::EvidenceFacts};

/// Absence grants nothing; a present private proof must pass its family checker.
pub(crate) fn presence<'a, T: 'a>(
    facts: &'a dyn EvidenceFacts,
    check: impl FnOnce(&'a dyn EvidenceFacts) -> Result<Option<&'a T>, String>,
) -> Result<bool, String> {
    Ok(check(facts)?.is_some())
}

/// Cover one exact deferred call without deleting or relaxing raw diagnostics.
pub(crate) fn deferred_call(facts: &HirFnFacts, call: &HirNode) -> Result<(), String> {
    let [finding] = facts.source_diagnostics.as_slice() else {
        return Err("Checked instance requires its exact deferred call finding roster".into());
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
            "Checked instance deferred call finding differs from its proved source site".into(),
        );
    }
    Ok(())
}
