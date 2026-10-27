//! Private instance authority plus independently verified constructor dependency.
use super::*;
use crate::source_evidence::verify_binding_read as formal_read;

use crate::{HirNode, slice_length_instances::CheckedProjection};

const BINDING_READ: &str = "Slice length changes an exact original formal read";

pub(super) struct Checked<'a> {
    pub instance: &'a CheckedProjection,
    pub receiver: &'a HirNode,
    pub minimum: &'a HirNode,
    pub message: &'a HirNode,
    pub unit: &'a HirNode,
    pub constructor: WireContract,
}

pub(super) fn verify(facts: &dyn EvidenceFacts) -> Result<Checked<'_>, String> {
    let instance = facts
        .slice_length_instance()
        .ok_or("Missing checked slice instance")?;
    let [length, minimum] = instance.condition().children.as_slice() else {
        return Err("Slice length comparison loses its exact two operands".into());
    };
    let [receiver] = instance.length_call().children.as_slice() else {
        return Err("Slice length loses its single saved receiver".into());
    };
    let [message] = instance.error_call().children.as_slice() else {
        return Err("Slice error constructor changes its sole message argument".into());
    };
    if length.hir != instance.length_call().hir
        || !crate::data_library::source_resolved(length, facts)?
    {
        return Err("Slice length lacks its registered exact library operation".into());
    }

    let unit = unit_success(instance, facts)?;
    for (node, formal) in [receiver, minimum, message]
        .into_iter()
        .zip(&instance.facts().signature.params)
    {
        formal_read(node, &formal.pat, BINDING_READ)?;
    }

    let helper = crate::helper_calls::verify_use(instance.error_call(), facts)?;
    let constructor = crate::helper_calls::named_errors::validate_shape(helper.summary())?.clone();
    if helper.summary().argument_types != [crate::data_aggregates::token(message)?]
        || helper.summary().result_type != crate::data_aggregates::token(instance.error_call())?
    {
        return Err("Slice length constructor changes its exact static interface".into());
    }
    Ok(Checked {
        instance,
        receiver,
        minimum,
        message,
        unit,
        constructor,
    })
}

fn unit_success<'a>(
    instance: &'a CheckedProjection,
    facts: &dyn EvidenceFacts,
) -> Result<&'a HirNode, String> {
    let success = crate::concrete_results::verify_constructor(instance.success(), facts)?;
    let failure = crate::concrete_results::verify_constructor(instance.failure(), facts)?;
    if success.construct != "data.result-ok"
        || failure.construct != "data.result-err"
        || failure.payload.hir != instance.error_call().hir
        || success.payload.kind != "expr_tup"
        || !success.payload.children.is_empty()
    {
        return Err("Slice length changes its complete Result constructors".into());
    }
    Ok(success.payload)
}
