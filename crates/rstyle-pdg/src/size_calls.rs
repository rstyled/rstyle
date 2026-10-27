//! Concrete zero-argument scalar size calls, preserving original call syntax.
use crate::{HirNode, call_summaries::Summary, data_library::Callee, unit_store::EvidenceFacts};
use serde::{Deserialize, Serialize};

pub const CAPABILITY: &str = "rust.native-prefix-readers";
pub const RESOLUTION: &str = "resolved_scalar_size_call";
pub(crate) const CONSTRUCT: &str = "data.scalar-size";
pub(crate) const KEY: &str = "rstyle.data:scalar-size";

/// Reuses the existing callee identity; only the instantiated argument is new.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    pub callee: Callee,
    pub argument: String,
}

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    match (node.kind.as_str(), node.res.as_ref()) {
        ("expr_call", Some(resolution)) if resolution.kind == RESOLUTION => {
            crate::extraction_profile::facts_support(facts, CAPABILITY)
        }
        _ => false,
    }
}

pub(crate) fn resolve(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Option<Summary>, String> {
    if !candidate(node, facts) {
        return Ok(None);
    }
    verify(node, facts)?;
    let dispatch = node.dispatch.as_ref().ok_or("Missing size call dispatch")?;
    Ok(Some(Summary {
        id: "rstyle.data.scalar-size".into(),
        version: "1".into(),
        callee_path: dispatch.op.clone(),
        callee_crate: "core".into(),
        argument_types: Vec::new(),
        result_type: "usize".into(),
        memory_effects: false,
        mutates_arguments: false,
        may_panic: false,
        always_returns: true,
        overflow_behavior: "not_applicable".into(),
    }))
}

fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<Capture, String> {
    let resolution = node
        .res
        .as_ref()
        .filter(|res| res.kind == RESOLUTION && res.target.is_none())
        .ok_or("Missing resolved scalar size call")?;
    let capture: Capture =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    let registry = crate::data_library::registry_for(facts).ok_or("Unpinned scalar size build")?;
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing scalar size dispatch")?;
    if !node.children.is_empty()
        || node.expansion.is_some()
        || node.lit.is_some()
        || dispatch.kind != "resolved_direct"
        || !dispatch.operand_tys.is_empty()
        || dispatch.result_ty != "usize"
        || crate::data_aggregates::token(node)? != "usize"
        || dispatch.op != capture.callee.path
        || !matches!(
            capture.callee.path.as_str(),
            "core::mem::size_of" | "std::mem::size_of"
        )
        || capture.callee.crate_name != "core"
        || capture.callee.crate_hash != registry.core_hash
        || capture.callee.implementation.is_some()
        || registry.definitions.get("std::mem::size_of") != Some(&capture.callee.definition_hash)
        || !matches!(capture.argument.as_str(), "u16" | "u32" | "u128")
    {
        return Err("Scalar size disagrees with its pinned concrete call".into());
    }

    source(node, facts, &capture.argument)?;
    Ok(capture)
}

fn source(node: &HirNode, facts: &dyn EvidenceFacts, argument: &str) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;
    let syn::Expr::Call(call) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Size requires original call syntax".into());
    };
    let syn::Expr::Path(path) = crate::data_aggregates::ungroup(&call.func) else {
        return Err("Size requires a resolved function path".into());
    };
    let names: Vec<_> = path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    if !call.args.is_empty()
        || path.qself.is_some()
        || !(names == ["size_of"]
            || names == ["core", "mem", "size_of"]
            || names == ["std", "mem", "size_of"])
        || path
            .path
            .segments
            .iter()
            .take(path.path.segments.len() - 1)
            .any(|part| !part.arguments.is_empty())
    {
        return Err("Size source is outside the bounded primitive invocation".into());
    }

    let last = path
        .path
        .segments
        .last()
        .ok_or("Missing size function name")?;
    let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return Err("Missing explicit scalar size argument".into());
    };
    if arguments.args.len() != 1
        || !matches!(&arguments.args[0], syn::GenericArgument::Type(syn::Type::Path(ty)) if ty.qself.is_none() && ty.path.is_ident(argument))
    {
        return Err("Size source and instantiated scalar argument disagree".into());
    }
    Ok(())
}

pub(crate) fn semantics(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<serde_json::Value, String> {
    let capture = verify(node, facts)?;
    let bytes = match capture.argument.as_str() {
        "u16" => 2,
        "u32" => 4,
        "u128" => 16,
        _ => return Err("Unsupported scalar size".into()),
    };
    Ok(
        serde_json::json!({"rule":"scalar-size-in-bytes", "argument":capture.argument,"value":bytes}),
    )
}
