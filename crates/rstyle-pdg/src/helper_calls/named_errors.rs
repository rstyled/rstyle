//! Complete fresh-error constructor helpers; effects and typed routes stay verified.
mod source;
pub(crate) use source::empty_scalar_contract;
#[cfg(test)]
mod tests;

use crate::{
    HirNode, copy_tagged_values::origins::WireContract, producer::PairView,
    unit_store::EvidenceFacts,
};

pub(crate) const CAPABILITY: &str = "rust.named-error-helper-calls";
pub(crate) const SUMMARY: &str = "rstyle.helper.named-error-constructor/1";

/// Structural routing only; no name or available summary grants permission.
pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    super::capability_candidate(facts, CAPABILITY, crate::named_error_payloads::candidate)
}

/// A complete produced/verified callee pair precedes summary publication. Recheck
/// the dedicated document semantics even in trusted-generated verification mode.
pub(crate) fn derive(
    facts: &dyn EvidenceFacts,
    pair: PairView<'_>,
) -> Result<Option<WireContract>, String> {
    if !candidate(facts) {
        return Ok(None);
    }
    if pair.cfg.extraction_evidence.status != crate::ExtractionStatus::Complete
        || !pair.pdg.memory_locations.is_empty()
    {
        return Err("Fresh error helper requires complete memory-free callee evidence".into());
    }
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        CAPABILITY,
        || true,
        "Fresh error helper summary requires its registered document policy",
    )?;

    crate::data_documents::named_error_payloads::verify(pair, facts)?;
    Ok(Some(source_contract(facts)?))
}

/// Exact formal-to-component source meaning; this alone grants no graph authority.
pub(crate) fn source_contract(facts: &dyn EvidenceFacts) -> Result<WireContract, String> {
    crate::source_evidence::source_contract(
        facts,
        crate::named_error_payloads::verify_function,
        source::derive,
    )
}

pub(super) fn summary(
    facts: &dyn EvidenceFacts,
    output: WireContract,
) -> Result<super::Summary, String> {
    let mut summary = super::tagged_conversions::summary(facts, output)?;
    summary.identity = SUMMARY.into();
    Ok(summary)
}

pub(crate) fn validate_shape(summary: &super::Summary) -> Result<&WireContract, String> {
    if summary.argument_types.len() == 1 {
        return super::tagged_conversions::validate_shape_as(summary, SUMMARY);
    }
    validate_scalar_shape(summary)
}

/// Unlike concrete From selection, these ordinary inherent calls retain every
/// existing source/body/signature/build check rather than replacing resolution.
pub(super) fn verify_dependency(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
    summary: &super::Summary,
) -> Result<bool, String> {
    if summary.identity != SUMMARY {
        return Ok(false);
    }
    if !crate::extraction_profile::facts_support(caller, CAPABILITY) || !candidate(callee) {
        return Err("Fresh error helper lacks registered caller/callee capabilities".into());
    }
    if summary.argument_types.len() == 2
        && !crate::extraction_profile::facts_support(
            caller,
            crate::named_error_payloads::SCALAR_CAPABILITY,
        )
    {
        return Err("Scalar named error helper requires its registered caller capability".into());
    }
    let contract = validate_shape(summary)?;
    if contract != &source_contract(callee)?
        || summary.argument_types != super::argument_types(callee)?
        || summary.result_type != super::result_type(callee)?
    {
        return Err("Fresh error helper summary changes exact source interface or routes".into());
    }

    let resolution = super::selected_resolution(node, caller)
        .ok_or("Fresh error helper lacks exact ordinary call resolution")?;
    super::check_signature(node, caller, &resolution, callee)?;
    crate::inherent_calls::verify(node, caller, callee, &resolution)?;
    Ok(true)
}

fn validate_scalar_shape(summary: &super::Summary) -> Result<&WireContract, String> {
    if summary.identity != SUMMARY
        || summary.argument_types != ["u32", "u32"]
        || summary.result_type.is_empty()
        || summary.result_dependence != super::tagged_conversions::RESULT_DEPENDENCE
        || summary.argument_evaluation != "source-order-once/1"
        || summary.mutates_arguments
        || summary.memory_effects
        || summary.may_panic
        || !summary.always_returns
        || summary.bounded_loop.is_some()
        || summary.reference_outputs.is_some()
        || summary.component_outputs.is_some()
        || summary.memory_reads.is_some()
    {
        return Err("Scalar named error helper changes its complete constructor contract".into());
    }
    let contract = summary
        .tagged_output
        .as_ref()
        .ok_or("Scalar named error lacks its explicit empty reference cases")?;
    if !source::empty_scalar_contract(contract) {
        return Err(
            "Scalar named error helper invents reference components or alternatives".into(),
        );
    }
    Ok(contract)
}
