//! Complete checked slice-length helpers, with conditional static-error routes.
use crate::{
    HirNode, copy_tagged_values::origins::WireContract, producer::PairView,
    unit_store::EvidenceFacts,
};

#[cfg(test)]
mod tests;

pub(crate) const CAPABILITY: &str = "rust.named-error-reader-flow";
pub(crate) const SUMMARY: &str = "rstyle.helper.checked-slice-length/1";

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && facts.slice_length_instance().is_some()
}

/// Source routes alone never authorize publication; replay the complete pair.
pub(crate) fn derive(
    facts: &dyn EvidenceFacts,
    pair: PairView<'_>,
) -> Result<Option<WireContract>, String> {
    if !candidate(facts) {
        return Ok(None);
    }
    crate::data_documents::slice_lengths::verified_contract(pair, facts).map(Some)
}

pub(crate) fn source_contract(facts: &dyn EvidenceFacts) -> Result<WireContract, String> {
    crate::data_documents::slice_lengths::source_contract(facts)
}

pub(super) fn summary(
    facts: &dyn EvidenceFacts,
    output: WireContract,
) -> Result<super::Summary, String> {
    let checked = facts
        .slice_length_instance()
        .ok_or("Missing checked slice instance")?;
    let mut summary = super::tagged_conversions::summary(checked.facts(), output)?;
    summary.identity = SUMMARY.into();

    validate_shape(&summary)?;
    Ok(summary)
}

pub(crate) fn validate_shape(summary: &super::Summary) -> Result<&WireContract, String> {
    if summary.identity != SUMMARY
        || summary.argument_types.len() != 3
        || summary.argument_types.iter().any(String::is_empty)
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
        return Err("Slice-length helper changes its complete three-input contract".into());
    }
    summary
        .tagged_output
        .as_ref()
        .ok_or_else(|| "Slice-length helper lacks its exact conditional component contract".into())
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
    if !crate::extraction_profile::facts_support(caller, CAPABILITY) || !candidate(callee) {
        return Err("Slice-length helper lacks registered caller and checked callee".into());
    }
    let checked = callee
        .slice_length_instance()
        .ok_or("Missing checked slice instance")?;
    let original = callee
        .slice_length_original()
        .ok_or("Missing retained slice-length owner")?;
    if !crate::slice_length_instances::requested_call(original, caller.facts(), node)? {
        return Err("Slice-length summary lacks its exact original concrete call receipt".into());
    }
    if validate_shape(summary)? != &source_contract(callee)?
        || summary.argument_types != super::argument_types(checked.facts())?
        || summary.result_type != super::result_type(checked.facts())?
    {
        return Err("Slice-length helper changes original source interface or routes".into());
    }

    signature(node, caller, callee)?;
    Ok(true)
}

/// This view comes from an owner-installed exact receipt, never raw JSON alone.
pub(super) fn resolution(node: &HirNode, caller: &dyn EvidenceFacts) -> Option<serde_json::Value> {
    let helper = selected(node, caller).ok()??;
    let frontend = helper.frontend();
    let original = frontend.slice_length_original()?;
    super::local_resolution(original)
}

pub(super) fn callee(node: &HirNode, caller: &dyn EvidenceFacts) -> Option<String> {
    let helper = selected(node, caller).ok()??;
    Some(helper.frontend().name.clone())
}

fn selected<'a>(
    node: &HirNode,
    caller: &'a dyn EvidenceFacts,
) -> Result<Option<crate::unit_store::HelperView<'a>>, String> {
    if !crate::extraction_profile::facts_support(caller, CAPABILITY) {
        return Ok(None);
    }
    let Some(helper) = caller.helper(node)? else {
        return Ok(None);
    };

    let frontend = helper.frontend();
    if !candidate(&frontend) || helper.summary().identity != SUMMARY {
        return Ok(None);
    }

    let original = frontend
        .slice_length_original()
        .ok_or("Missing retained slice owner")?;
    if !crate::slice_length_instances::requested_call(original, caller.facts(), node)? {
        return Err("Slice helper selection differs from its exact concrete call receipt".into());
    }
    signature(node, caller, &frontend)?;
    Ok(Some(helper))
}

fn signature(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::slice_length_instances::activation::verify(callee)?;
    let checked = callee
        .slice_length_instance()
        .ok_or("Missing checked slice instance")?;
    let expected = super::argument_types(checked.facts())?;

    if node.kind != "expr_call"
        || node.expansion.is_some()
        || node.is_unsafe_block
        || node.children.len() != expected.len()
        || node
            .children
            .iter()
            .zip(&expected)
            .any(|(actual, expected)| {
                actual.ty.as_ref().and_then(|ty| ty.normalized.as_ref()) != Some(expected)
            })
        || node.ty.as_ref().and_then(|ty| ty.normalized.as_ref())
            != checked.facts().signature.return_ty.normalized.as_ref()
        || !super::same_build(caller, callee)?
    {
        return Err(
            "Slice helper changes exact actual types, arity, result or owning build".into(),
        );
    }
    Ok(())
}
