//! Exact checked conversion summaries. Tagged alternatives retain nominal routes.
//! Callers verify the complete callee pair before publishing or consuming a summary.
use crate::{
    HirNode, copy_tagged_values::origins::WireContract, producer::PairView,
    unit_store::EvidenceFacts,
};

#[cfg(test)]
mod tests;

pub(crate) const CAPABILITY: &str = "rust.copy-tagged-conversion-helper-calls";
pub(crate) const SUMMARY: &str = "rstyle.helper.copy-tagged-conversion/1";
pub(crate) const RESULT_DEPENDENCE: &str = "verified-guarded-component-substitution/1";

/// Candidate routing only; this does not establish graph or helper-call authority.
pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && crate::copy_tagged_values::candidate(facts)
}

pub(crate) fn supported(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if !candidate(facts) {
        return Ok(false);
    }
    Ok(facts
        .nominal_resolution()?
        .copy_conversion_source(facts.facts())?
        .is_some())
}

/// Publish only after normal producer verification of this complete callee pair.
/// The graph verifier replays saved values and every typed reference-bearing role;
/// deriving a source contract alone is deliberately insufficient.
pub(crate) fn derive(
    facts: &dyn EvidenceFacts,
    pair: PairView<'_>,
) -> Result<Option<WireContract>, String> {
    if !candidate(facts) {
        return Ok(None);
    }
    let resolution = facts.nominal_resolution()?;
    let Some(receipt) = resolution.copy_conversion_source(facts.facts())? else {
        return Ok(None);
    };

    if pair.cfg.extraction_evidence.status != crate::ExtractionStatus::Complete
        || !pair.pdg.memory_locations.is_empty()
    {
        return Err("Tagged helper requires complete memory-free callee evidence".into());
    }
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        CAPABILITY,
        || true,
        "Tagged helper summary requires its registered document policy",
    )?;

    crate::data_documents::copy_tagged_values::verify(pair, facts, receipt)?;
    crate::data_documents::copy_tagged_values::origins::verify(pair, facts, receipt)?;
    let contract = crate::copy_tagged_values::origins::derive_source_contract(facts, receipt)?;
    Ok(Some(WireContract::from_contract(&contract)))
}

/// Validate a summary from an independently verified callee dependency. Exact
/// saved actuals and caller graph return/guard composition remain graph obligations.
pub(super) fn verify_dependency(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
    summary: &super::Summary,
) -> Result<bool, String> {
    if !super::tagged_resolution::verify(node, caller, callee)? {
        if summary.tagged_output.is_some() {
            return Err("Tagged helper output has no exact selected conversion call".into());
        }
        return Ok(false);
    }
    if !crate::extraction_profile::facts_support(caller, CAPABILITY) || !candidate(callee) {
        return Err("Tagged helper call requires its registered caller and checked callee".into());
    }

    let output = validate_shape(summary)?;
    let resolution = callee.nominal_resolution()?;
    let receipt = resolution
        .copy_conversion_source(callee.facts())?
        .ok_or("Tagged helper call lacks its retained original callee")?;

    crate::copy_tagged_values::origins::validate_wire_source_contract(output, callee, receipt)?;
    Ok(true)
}

/// The contract comes exclusively from `derive` after full source and graph replay.
pub(super) fn summary(
    facts: &dyn EvidenceFacts,
    output: WireContract,
) -> Result<super::Summary, String> {
    Ok(super::Summary {
        identity: SUMMARY.into(),
        argument_types: super::argument_types(facts)?,
        result_type: super::result_type(facts)?,
        mutates_arguments: false,
        memory_effects: false,
        may_panic: false,
        always_returns: true,
        argument_evaluation: "source-order-once/1".into(),
        result_dependence: RESULT_DEPENDENCE.into(),
        bounded_loop: None,
        reference_outputs: None,
        component_outputs: None,
        memory_reads: None,
        tagged_output: Some(output),
    })
}

pub(super) fn validate_shape(summary: &super::Summary) -> Result<&WireContract, String> {
    if summary.identity != SUMMARY
        || summary.result_dependence != RESULT_DEPENDENCE
        || summary.argument_evaluation != "source-order-once/1"
        || summary.argument_types.len() != 1
        || summary.argument_types[0].is_empty()
        || summary.result_type.is_empty()
        || summary.mutates_arguments
        || summary.memory_effects
        || summary.may_panic
        || !summary.always_returns
        || summary.bounded_loop.is_some()
        || summary.reference_outputs.is_some()
        || summary.component_outputs.is_some()
        || summary.memory_reads.is_some()
    {
        return Err("Tagged helper summary changes its complete conversion contract".into());
    }
    summary
        .tagged_output
        .as_ref()
        .ok_or_else(|| "Tagged helper summary lacks its exact guarded output contract".into())
}
