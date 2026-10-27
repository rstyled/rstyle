//! Complete ID readers publish read and panic effects alongside exact static error routes.
use crate::{
    HirNode, copy_tagged_values::origins::WireContract, producer::PairView,
    unit_store::EvidenceFacts,
};

pub(crate) const CAPABILITY: &str = "rust.named-error-id-reader-flow";
pub(crate) const SUMMARY: &str = "rstyle.helper.id-reader-read-only-body/1";

const FAMILY: super::source_family::Family = super::source_family::Family {
    capability: CAPABILITY,
    candidate: crate::data_documents::id_readers::candidate,
    source: crate::data_documents::id_readers::verify_source,
    identity: SUMMARY,
    validate: validate_shape,
};

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    FAMILY.candidate(facts)
}

pub(crate) fn supported(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    FAMILY.supported(facts)
}

pub(crate) fn derive(
    facts: &dyn EvidenceFacts,
    pair: PairView<'_>,
) -> Result<Option<WireContract>, String> {
    if !candidate(facts) {
        return Ok(None);
    }
    if !crate::data_documents::id_readers::verify(pair, facts)? {
        return Err(
            "ID reader helper requires whole source, bounds, panic and origin replay".into(),
        );
    }
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        CAPABILITY,
        || true,
        "ID reader helper requires its registered document policy",
    )?;

    crate::data_documents::id_readers::source_contract(facts).map(Some)
}

pub(super) fn summary(
    facts: &dyn EvidenceFacts,
    pair: &crate::producer::Pair,
    output: WireContract,
) -> Result<super::Summary, String> {
    FAMILY.summary(facts, pair, output)
}

pub(crate) fn validate_shape(summary: &super::Summary) -> Result<&WireContract, String> {
    if summary.identity != SUMMARY
        || summary.argument_types.len() != 2
        || summary.argument_types.iter().any(String::is_empty)
        || summary.result_type.is_empty()
        || summary.argument_evaluation != "source-order-once/1"
        || summary.result_dependence != super::tagged_conversions::RESULT_DEPENDENCE
        || summary.mutates_arguments
        || !summary.memory_effects
        || summary.memory_reads.as_ref().is_none_or(Vec::is_empty)
        || !summary.may_panic
        || summary.always_returns
        || summary.bounded_loop.is_some()
        || summary.reference_outputs.is_some()
        || summary.component_outputs.is_some()
    {
        return Err("ID reader helper changes exact read/panic/ownership contract".into());
    }
    summary
        .tagged_output
        .as_ref()
        .ok_or_else(|| "ID reader helper lacks exact output cases and static routes".into())
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
        return Err(
            "ID reader helper lacks its registered caller and independently checked callee".into(),
        );
    }
    if validate_shape(summary)? != &crate::data_documents::id_readers::source_contract(callee)?
        || summary.argument_types != super::argument_types(callee)?
        || summary.result_type != super::result_type(callee)?
    {
        return Err("ID reader helper changes exact selected interface or static routes".into());
    }
    let resolution = super::selected_resolution(node, caller)
        .ok_or("ID reader helper lacks exact ordinary source selection")?;
    super::check_signature(node, caller, &resolution, callee)?;
    crate::inherent_calls::verify(node, caller, callee, &resolution)?;

    Ok(true)
}
