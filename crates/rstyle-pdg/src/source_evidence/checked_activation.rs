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
    if facts.source_scan_complete {
        return Err(
            "Checked instance deferred call finding differs from its proved source site".into(),
        );
    }
    exact_deferred_call(facts, call)
}

/// Exact deferred finding coverage, independent of the producer's source-scan
/// completeness flag. Callers separately prove their complete bounded source.
pub(crate) fn exact_deferred_call(facts: &HirFnFacts, call: &HirNode) -> Result<(), String> {
    exact_deferred_calls(facts, &[call])
}

/// Whole families enumerate every deferred source call; none may be omitted.
pub(crate) fn exact_deferred_calls(facts: &HirFnFacts, calls: &[&HirNode]) -> Result<(), String> {
    if facts.source_diagnostics.len() != calls.len() {
        return Err("Checked instance requires its exact deferred call finding roster".into());
    }
    for (finding, call) in facts.source_diagnostics.iter().zip(calls) {
        deferred_finding(finding, call)?;
    }
    Ok(())
}

pub(crate) fn deferred_finding(
    finding: &crate::HirDiagnostic,
    call: &HirNode,
) -> Result<(), String> {
    if finding.capability != "rust.call-summaries"
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
