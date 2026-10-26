//! Guarded component outputs compose with verified read-only normal-flow effects.
use crate::{
    HirNode, copy_tagged_values::origins::WireContract, producer::PairView,
    unit_store::EvidenceFacts,
};

#[cfg(test)]
mod tests;

pub(crate) const CAPABILITY: &str = "rust.named-error-validator-flow";
pub(crate) const SUMMARY: &str = "rstyle.helper.guarded-read-only-body/1";

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && crate::data_documents::fallible_readers::candidate(facts)
}

/// Checked no-drop results are separate from Copy interfaces. Source proof here
/// grants no permission to publish a summary without complete graph replay.
pub(crate) fn supported(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    Ok(candidate(facts) && crate::data_documents::fallible_readers::verify_source(facts)?)
}

pub(crate) fn derive(
    facts: &dyn EvidenceFacts,
    pair: PairView<'_>,
) -> Result<Option<WireContract>, String> {
    if !candidate(facts) {
        return Ok(None);
    }
    if !crate::data_documents::fallible_readers::verify(pair, facts)? {
        return Err("Guarded helper lacks its whole source/graph origin proof".into());
    }
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        CAPABILITY,
        || true,
        "Guarded read-only helper requires its registered document policy",
    )?;
    source_contract(facts).map(Some)
}

fn source_contract(facts: &dyn EvidenceFacts) -> Result<WireContract, String> {
    crate::data_documents::fallible_readers::source_contract(facts)
}

pub(super) fn summary(
    facts: &dyn EvidenceFacts,
    pair: &crate::producer::Pair,
    output: WireContract,
) -> Result<super::Summary, String> {
    let mut summary = super::legacy_summary(facts, pair, None)?;

    summary.identity = SUMMARY.into();
    summary.result_dependence = super::tagged_conversions::RESULT_DEPENDENCE.into();
    summary.tagged_output = Some(output);

    validate_shape(&summary)?;
    Ok(summary)
}

pub(crate) fn validate_shape(summary: &super::Summary) -> Result<&WireContract, String> {
    if summary.identity != SUMMARY
        || summary.argument_types.is_empty()
        || summary.argument_types.iter().any(String::is_empty)
        || summary.result_type.is_empty()
        || summary.argument_evaluation != "source-order-once/1"
        || summary.result_dependence != super::tagged_conversions::RESULT_DEPENDENCE
        || summary.mutates_arguments
        || !summary.memory_effects
        || summary.memory_reads.as_ref().is_none_or(Vec::is_empty)
        || summary.always_returns == summary.may_panic
        || summary.bounded_loop.is_some()
        || summary.reference_outputs.is_some()
        || summary.component_outputs.is_some()
    {
        return Err("Guarded helper changes its read-only normal-flow contract".into());
    }
    summary
        .tagged_output
        .as_ref()
        .ok_or_else(|| "Guarded helper lacks its explicit output cases and component routes".into())
}

pub(super) fn verify_dependency(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
    summary: &super::Summary,
) -> Result<bool, String> {
    if summary.identity != SUMMARY {
        return Ok(false);
    }
    if !crate::extraction_profile::facts_support(caller, CAPABILITY) || !supported(callee)? {
        return Err("Guarded helper lacks its registered caller and checked no-drop callee".into());
    }

    if validate_shape(summary)? != &source_contract(callee)?
        || summary.argument_types != super::argument_types(callee)?
        || summary.result_type != super::result_type(callee)?
    {
        return Err("Guarded helper changes its original interface or output routes".into());
    }
    let resolution = super::selected_resolution(node, caller)
        .ok_or("Guarded helper lacks its exact ordinary call selection")?;
    super::check_signature(node, caller, &resolution, callee)?;
    crate::inherent_calls::verify(node, caller, callee, &resolution)?;
    Ok(true)
}
