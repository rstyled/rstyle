//! Complete original checked u32 widening readers; no general callback permission.
mod authority;
mod environments;
mod roles;
mod source;

use crate::{HirNode, unit_store::EvidenceFacts};

pub(crate) const RESULT: &str = "data.widening-reader.result";
pub(crate) const OUTER_ENVIRONMENT: &str = "data.widening-reader.outer-environment";
pub(crate) const OUTER_SELECTOR: &str = "data.widening-reader.outer-is-ok";
pub(crate) const READER_PAYLOAD: &str = "data.widening-reader.reader-ok-payload";
pub(crate) const DECODED_PAYLOAD: &str = "data.widening-reader.decoded-payload";
pub(crate) const COUNT_PAYLOAD: &str = "data.widening-reader.count-payload";
pub(crate) const BINDING: &str = "data.widening-reader.binding";
pub(crate) const READER_ERROR: &str = "data.widening-reader.reader-error-forward";
pub(crate) const MAP_ENVIRONMENT: &str = "data.widening-reader.map-environment";
pub(crate) const MAP_SELECTOR: &str = "data.widening-reader.map-is-ok";
pub(crate) const CONVERSION_PAYLOAD: &str = "data.widening-reader.conversion-ok-payload";
pub(crate) const COUNT_CAPTURE: &str = "data.widening-reader.count-capture";
pub(crate) const SUCCESS_TUPLE: &str = "data.widening-reader.success-tuple";
pub(crate) const MAP_OK: &str = "data.widening-reader.map-ok";
pub(crate) const MAP_ERROR: &str = "data.widening-reader.map-error-forward";
pub(crate) const FAILURE_ENVIRONMENT: &str = "data.widening-reader.failure-environment";
pub(crate) const FAILURE_SELECTOR: &str = "data.widening-reader.failure-is-err";
pub(crate) const ERROR_DISCARD: &str = "data.widening-reader.conversion-error-discard";
pub(crate) const MESSAGE_CAPTURE: &str = "data.widening-reader.message-capture";
pub(crate) const FINAL_ERROR: &str = "data.widening-reader.final-error";
pub(crate) const FINAL_PAYLOAD: &str = "data.widening-reader.final-success-payload";
pub(crate) const FINAL_OK: &str = "data.widening-reader.final-ok";

pub(crate) const CAPABILITY: &str = "rust.named-error-usize-reader-flow";
pub(crate) struct Checked<'a> {
    pub roles: roles::Roles<'a>,
    pub proof: authority::Proof,
}

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && matches!(facts.body.children.as_slice(), [node]
            if node.kind == "expr_method_call"
                && node.dispatch.as_ref().is_some_and(|dispatch|
                    matches!(dispatch.op.as_str(),
                        "std::result::Result::<T, E>::and_then"
                        | "core::result::Result::<T, E>::and_then")))
}

pub(crate) fn node_candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    candidate(facts)
        && facts
            .body
            .children
            .first()
            .is_some_and(|source| source.hir == node.hir)
}

pub(crate) fn verify(facts: &dyn EvidenceFacts) -> Result<Checked<'_>, String> {
    if !candidate(facts) {
        return Err("Missing registered whole widening-reader source".into());
    }

    let roles = roles::verify(facts)?;
    findings(&roles, facts)?;

    let proof = authority::verify(&roles, facts)?;

    Ok(Checked { roles, proof })
}

pub(crate) fn verify_source(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if !candidate(facts) {
        return Ok(false);
    }
    verify(facts)?;

    Ok(true)
}

pub(crate) fn merge_candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    if !candidate(facts) {
        return false;
    }

    let Some(source) = facts.body.children.first() else {
        return false;
    };

    let nested = source
        .children
        .get(1)
        .and_then(|closure| closure.children.get(1))
        .and_then(|body| body.children.first());
    source.hir == node.hir
        || nested.is_some_and(|map_err| {
            map_err.hir == node.hir
                || map_err
                    .children
                    .first()
                    .is_some_and(|map| map.hir == node.hir)
        })
}

/// Scoped call authority exists only inside the independently proved whole family.
pub(crate) fn conversion_summary(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Option<crate::call_summaries::Summary>, String> {
    if !candidate(facts)
        || crate::lossless_integer_calls::kind(node, facts)
            != Some(crate::lossless_integer_calls::Kind::ConvertU32Usize)
    {
        return Ok(None);
    }

    let checked = verify(facts)?;

    if node.hir != checked.roles.conversion.hir || node.span != checked.roles.conversion.span {
        return Err("Widening conversion differs from its whole-function source role".into());
    }
    crate::lossless_integer_calls::verify_scoped_u32_conversion(
        node,
        &[checked.roles.continuation],
        facts,
    )
    .map(Some)
}

/// Scalar tuple ownership is independent of callbacks and never covers the opaque carrier.
pub(crate) fn success_shape(token: &crate::data_types::TypeToken) -> bool {
    matches!(token, crate::data_types::TypeToken::Tuple(parts)
        if parts.len() == 2 && parts.iter().all(|part|
            matches!(part, crate::data_types::TypeToken::Scalar(name) if name == "usize")))
}

fn findings(roles: &roles::Roles<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();

    for finding in &facts.source_diagnostics {
        if !seen.insert((
            finding.span.start,
            finding.span.end,
            finding.construct.as_str(),
        )) {
            return Err("Widening reader repeats a deferred source finding".into());
        }
        if let Some(call) = [roles.source, roles.map_err, roles.map]
            .into_iter()
            .find(|call| call.span == finding.span && call.snippet == finding.snippet)
        {
            crate::source_evidence::checked_activation::deferred_finding(finding, call)?;
        } else {
            binding_finding(finding, roles)?;
        }
    }

    Ok(())
}

fn binding_finding(finding: &crate::HirDiagnostic, roles: &roles::Roles<'_>) -> Result<(), String> {
    if finding.capability != "rust.scalar-interface"
        || finding.construct != "binding_pattern"
        || finding.severity != "error"
        || finding.reason
            != "Source construct is deferred to rust.scalar-interface; remove it or assess it under a later policy."
        || ![roles.payload, roles.wildcard]
            .iter()
            .any(|pattern| finding.span == pattern.span && finding.snippet == pattern.snippet)
    {
        return Err("Widening reader has an uncovered original source finding".into());
    }

    Ok(())
}
