//! Fresh no-drop error values and eager checked-reference Option conversion.
mod authority;
mod error;
mod source;
#[cfg(test)]
mod tests;

use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
pub(crate) use error::{error_candidate, error_semantics, verify_error};

pub(crate) const CAPABILITY: &str = "rust.checked-borrowed-result-flow";
pub(crate) const KEY: &str = "rstyle.data:borrowed-result-error";
pub(crate) const CONSTRUCT: &str = "data.option-ok-or";
pub(crate) const ERROR: &str = "data.fresh-error-variant";
const METHOD: &str = "std::option::Option::<T>::ok_or";
const CORE_METHOD: &str = "core::option::Option::<T>::ok_or";

pub(crate) struct Checked<'a> {
    pub receiver: &'a HirNode,
    pub error: &'a HirNode,
}

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::gated_node(node, facts, CAPABILITY, |node| {
        node.kind == "expr_method_call"
            && node
                .dispatch
                .as_ref()
                .is_some_and(|dispatch| matches!(dispatch.op.as_str(), METHOD | CORE_METHOD))
    })
}

pub(crate) fn verify<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Checked<'a>, String> {
    if !candidate(node, facts) || node.expansion.is_some() {
        return Err("Eager ok_or requires its original registered method invocation".into());
    }
    let [receiver, error] = node.children.as_slice() else {
        return Err("Eager ok_or requires receiver then one fresh error operand".into());
    };
    authority::verify(node, facts)?;
    source::method(node, facts)?;
    let checked = verify_error(error, facts)?;
    let input = crate::data_aggregates::parsed_type(receiver)?;
    let output = crate::data_aggregates::parsed_type(node)?;
    types(&input, &checked.token, &output)?;
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing eager ok_or signature")?;
    if dispatch.kind != "resolved_method"
        || dispatch.operand_tys
            != [
                crate::data_aggregates::token(receiver)?,
                crate::data_aggregates::token(error)?,
            ]
        || dispatch.result_ty != crate::data_aggregates::token(node)?
    {
        return Err(
            "Eager ok_or dispatch disagrees with its complete concrete operands/result".into(),
        );
    }

    Ok(Checked { receiver, error })
}

fn types(input: &TypeToken, error: &TypeToken, output: &TypeToken) -> Result<(), String> {
    let TypeToken::Option(payload) = input else {
        return Err("Eager ok_or requires a concrete checked Option".into());
    };
    if !matches!(payload.as_ref(), TypeToken::Shared(slice)
        if matches!(slice.as_ref(), TypeToken::Slice(byte)
            if matches!(byte.as_ref(), TypeToken::Scalar(value) if value == "u8")))
        || !matches!(output, TypeToken::Result { ok, err } if ok == payload && err.as_ref() == error)
    {
        return Err(
            "Eager ok_or requires an exact shared byte-slice success and saved error type".into(),
        );
    }

    Ok(())
}

pub(crate) fn resolve(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<crate::call_summaries::Summary, String> {
    verify(node, facts)?;
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing eager ok_or dispatch")?;

    Ok(crate::call_summaries::Summary {
        id: "rstyle.data.option.ok-or-fresh-error".into(),
        version: "1".into(),
        callee_path: dispatch.op.clone(),
        callee_crate: "core".into(),
        argument_types: dispatch.operand_tys.clone(),
        result_type: dispatch.result_ty.clone(),
        memory_effects: false,
        mutates_arguments: false,
        may_panic: false,
        always_returns: true,
        overflow_behavior: "not_applicable".into(),
    })
}

pub(crate) fn sources(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::field_locations::selected_operations(facts, &["expr_method_call"], candidate)
}

pub(crate) fn errors(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::field_locations::selected_operations(facts, &["expr_path"], error_candidate)
}

#[derive(serde::Serialize)]
struct Semantics {
    rule: &'static str,
    receiver_operand: u64,
    error_operand: u64,
    evaluation: &'static str,
    some: &'static str,
    none: &'static str,
    unused_error: &'static str,
    content_read: bool,
}

pub(crate) fn semantics() -> serde_json::Value {
    let semantics = Semantics {
        rule: "eager-option-to-result",
        receiver_operand: 0,
        error_operand: 1,
        evaluation: "receiver-then-error-once-even-some",
        some: "ok-saved-shared-payload",
        none: "err-saved-fresh-error",
        unused_error: "independently-source-proved-no-drop",
        content_read: false,
    };

    serde_json::to_value(semantics).expect("registered eager error semantics")
}
