//! Exact builtin bool-to-usize conversion; no general scalar-cast permission.
use crate::{HirNode, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

pub(crate) const CAPABILITY: &str = "rust.bool-to-usize";
pub(crate) const CONSTRUCT: &str = "data.bool-to-usize";
pub(crate) const KEY: &str = "rstyle.data:bool-to-usize";

pub(crate) fn candidate(node: &HirNode) -> bool {
    node.kind == "expr_cast"
        && node.children.first().and_then(crate::frontend::normalized) == Some("bool")
}

pub(crate) fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || crate::data_library::registry_for(facts).is_none()
        || facts.build.pointer_width.value != Some(64)
    {
        return Err(
            "Bool-to-usize requires its registered policy and pinned compiler/target".into(),
        );
    }

    let [operand] = node.children.as_slice() else {
        return Err("Bool cast requires one saved operand".into());
    };
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing builtin bool cast dispatch")?;
    if !candidate(node)
        || crate::frontend::normalized(node) != Some("usize")
        || dispatch.kind != "builtin"
        || dispatch.op != "as"
        || dispatch.operand_tys != ["bool"]
        || dispatch.result_ty != "usize"
        || node.expansion.is_some()
        || node.is_unsafe_block
    {
        return Err(
            "Bool-to-usize requires exact original builtin bool/usize types and dispatch".into(),
        );
    }

    let syn::Expr::Cast(expression) = crate::source_evidence::original_expression(node, facts)?
    else {
        return Err("Missing original bool cast syntax".into());
    };
    if !matches!(expression.ty.as_ref(), syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("usize"))
    {
        return Err("Bool-to-usize requires the exact builtin usize spelling".into());
    }

    crate::source_evidence::verify_child_span(node, operand, expression.expr.span(), facts)
}

pub(crate) fn semantics(
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<serde_json::Value, String> {
    verify(source, facts)?;
    let interpretation = Interpretation {
        identity: "rstyle.rust.bool-to-usize/1",
        input: "bool",
        result: "usize",
        false_value: "0",
        true_value: "1",
        evaluation: "one ordered saved operand",
        failure: "none",
        pointer_width: 64,
        rustc_commit: &facts.build.rustc_commit,
        target: &facts.build.target_triple,
    };
    serde_json::to_value(interpretation).map_err(|error| error.to_string())
}

#[derive(serde::Serialize)]
struct Interpretation<'a> {
    identity: &'a str,
    input: &'a str,
    result: &'a str,
    #[serde(rename = "false")]
    false_value: &'a str,
    #[serde(rename = "true")]
    true_value: &'a str,
    evaluation: &'a str,
    failure: &'a str,
    pointer_width: u32,
    rustc_commit: &'a crate::StringFact,
    target: &'a crate::StringFact,
}
