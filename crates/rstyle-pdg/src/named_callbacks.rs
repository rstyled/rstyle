//! Immediate compiler-resolved function items; no general callable storage.
use crate::{HirNode, helper_calls, option_callbacks, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.named-option-callbacks";
pub(crate) const ITEM: &str = "callback.function-item";
pub(crate) const SELECTOR: &str = "callback.named-option-some";
pub(crate) const PAYLOAD: &str = "callback.named-option-payload";
pub(crate) const INVOKE: &str = "callback.named-invoke";
pub(crate) const RESULT: &str = "callback.named-option-result";

pub(crate) fn function_item(node: &HirNode) -> bool {
    node.kind == "expr_path"
        && node
            .res
            .as_ref()
            .is_some_and(|res| res.kind == "resolved_function_item")
}

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && option_callbacks::candidate(node, facts)
        && node.children.last().is_some_and(function_item)
}

pub(crate) fn owner<'a>(
    item: &HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<&'a HirNode, String> {
    let calls: Vec<_> = option_callbacks::invocations(facts, option_callbacks::Kind::FunctionItem)
        .into_iter()
        .filter(|call| {
            call.children
                .last()
                .is_some_and(|callback| callback.hir == item.hir && callback.span == item.span)
        })
        .collect();
    let [call] = calls.as_slice() else {
        return Err("Function item lacks one immediate map_or owner".into());
    };
    Ok(call)
}

pub(crate) fn verify<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<option_callbacks::Arguments<'a>, String> {
    let arguments = verify_source(node, facts)?;
    let dependency = helper_calls::verify_use(arguments.callback, facts)?;
    verify_summary(dependency.summary())?;
    Ok(arguments)
}

/// Source correspondence runs before unit helper linking. It recognizes the
/// complete immediate syntax; eligibility separately requires the callee proof.
pub(crate) fn verify_source<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<option_callbacks::Arguments<'a>, String> {
    if !candidate(node, facts) {
        return Err("Named callback requires its registered immediate context".into());
    }
    let arguments = option_callbacks::verify_arguments(node, facts)?;
    verify_item(arguments.callback, facts)?;
    Ok(arguments)
}

pub(crate) fn verify_item(item: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !function_item(item)
        || item.expansion.is_some()
        || !item.children.is_empty()
        || item.ty.as_ref().is_none_or(|ty| ty.normalized.is_some())
    {
        return Err("Named callback requires an uncoerced compiler FnDef path".into());
    }

    owner(item, facts)?;
    let syntax = crate::source_evidence::original_expression(item, facts)?;
    let syn::Expr::Path(path) = syntax else {
        return Err("Function item requires its original path".into());
    };
    if path.qself.is_some()
        || !path.attrs.is_empty()
        || path
            .path
            .segments
            .iter()
            .any(|segment| !segment.arguments.is_empty())
    {
        return Err("Named callback requires a nongeneric resolved function path".into());
    }
    Ok(())
}

pub(crate) fn signature(
    item: &HirNode,
    facts: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
) -> Result<(), String> {
    verify_item(item, facts)?;
    let signature = &callee.signature;
    if signature.has_self
        || signature.is_async
        || signature.is_unsafe
        || signature.is_variadic
        || !signature.generics.is_empty()
        || signature.abi.known_value() != Some("Rust")
        || signature.params.len() != 1
        || signature.params[0].ty.normalized.as_deref() != Some("u8")
        || signature.return_ty.normalized.as_deref() != Some("bool")
        || item.dispatch.as_ref().is_none_or(|dispatch| {
            dispatch.kind != "resolved_function_item"
                || dispatch.operand_tys != ["u8"]
                || dispatch.result_ty != "bool"
        })
    {
        return Err(
            "Named callback must resolve the complete safe Rust fn(u8) -> bool signature".into(),
        );
    }
    Ok(())
}

pub(crate) fn verify_summary(summary: &helper_calls::Summary) -> Result<(), String> {
    if summary.identity != "rstyle.helper.source-body/1"
        || summary.argument_types != ["u8"]
        || summary.result_type != "bool"
        || summary.mutates_arguments
        || summary.memory_effects
        || summary.may_panic
        || !summary.always_returns
        || summary.reference_outputs.is_some()
        || summary.memory_reads.is_some()
    {
        return Err("Named callback requires a fully verified returning scalar helper without effects or panic".into());
    }
    Ok(())
}
