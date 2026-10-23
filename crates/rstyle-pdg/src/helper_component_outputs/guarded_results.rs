//! Guarded reference products retain the complete saved callee result graph.
use super::*;
use crate::helper_calls::{Summary, VerifiedDependency};
#[cfg(test)]
mod tests;

pub(crate) const CAPABILITY: &str = "rust.linear-product-result-flow";
pub(crate) const SUMMARY: &str = "rstyle.helper.guarded-product-result/1";
pub(crate) const RESULT_DEPENDENCE: &str =
    "guarded-saved-result-with-verified-callee-return-graph/1";

pub(crate) struct CheckedProductCall {
    pub(crate) result: TypeToken,
    pub(crate) ok: TypeToken,
    pub(crate) error: TypeToken,
}

pub(super) fn shape(ty: &TypeToken) -> bool {
    matches!(ty, TypeToken::Result { ok, err }
        if crate::concrete_results::fresh_products::GRAMMAR.permits(ok, err))
}

pub(super) fn supported(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && crate::concrete_results::fresh_products::product_boundary(facts).is_ok()
}

pub(crate) fn verify_product_call(
    call: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<CheckedProductCall, String> {
    let dependency = crate::helper_calls::verify_use(call, facts)?;
    check(call, facts, &dependency)
}

pub(super) fn verify_dependency(
    call: &HirNode,
    facts: &dyn EvidenceFacts,
    dependency: &VerifiedDependency<'_>,
) -> Result<(), String> {
    if shape(&crate::data_aggregates::parsed_type(call)?) {
        check(call, facts, dependency)?;
    }
    Ok(())
}

fn check(
    call: &HirNode,
    facts: &dyn EvidenceFacts,
    dependency: &VerifiedDependency<'_>,
) -> Result<CheckedProductCall, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || !crate::extraction_profile::facts_support(facts, super::CAPABILITY)
    {
        return Err("Guarded product helper lacks its registered capabilities".into());
    }
    let actual = crate::data_aggregates::parsed_type(call)?;
    verify_summary(dependency.summary(), &actual)?;
    match dependency {
        VerifiedDependency::Shared(view) => boundary(facts, &view.frontend(), actual),
        VerifiedDependency::Legacy(dependency) => boundary(facts, &dependency.frontend, actual),
    }
}

fn boundary(
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
    actual: TypeToken,
) -> Result<CheckedProductCall, String> {
    if !crate::helper_references::same_unit(caller, callee)?
        || !crate::extraction_profile::facts_support(callee, CAPABILITY)
    {
        return Err("Guarded product helpers require the same checked selected unit".into());
    }
    let checked = crate::concrete_results::fresh_products::product_boundary(callee)?;
    if checked.result != actual {
        return Err("Guarded helper result differs from its checked product boundary".into());
    }
    Ok(CheckedProductCall {
        result: checked.result,
        ok: checked.ok,
        error: checked.error,
    })
}

fn verify_summary(summary: &Summary, actual: &TypeToken) -> Result<(), String> {
    if !shape(actual)
        || summary.identity != SUMMARY
        || summary.result_type != actual.text()?
        || summary.argument_evaluation != "source-order-once/1"
        || summary.result_dependence != RESULT_DEPENDENCE
        || summary.mutates_arguments
        || summary.memory_effects
        || summary.may_panic
        || !summary.always_returns
        || summary.memory_reads.is_some()
        || summary.bounded_loop.is_some()
        || summary.reference_outputs.is_some()
    {
        return Err("Guarded product helper lacks its complete saved-result contract".into());
    }
    roster::verify(
        actual,
        summary
            .component_outputs
            .as_deref()
            .ok_or("Missing guarded component outputs")?,
    )
}
