//! A complete scalar Some condition with explicit success and failure branches.

use super::*;
use crate::data_types::TypeToken;

pub(crate) const CAPABILITY: &str = "rust.scalar-option-if-let";

pub(crate) struct IfLet<'a> {
    pub source: &'a HirNode,
    pub condition: &'a HirNode,
    pub scrutinee: &'a HirNode,
    pub binding: &'a HirNode,
    pub then_branch: &'a HirNode,
    pub else_branch: &'a HirNode,
}

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && node.kind == "expr_if"
        && node
            .children
            .first()
            .is_some_and(|node| node.kind == "expr_let")
}

pub(crate) fn verify<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<IfLet<'a>, String> {
    if !candidate(node, facts) || node.expansion.is_some() || node.is_unsafe_block {
        return Err("Scalar Option if-let requires its registered original condition".into());
    }
    let [condition, then_branch, else_branch] = node.children.as_slice() else {
        return Err("Scalar Option if-let requires both complete branches".into());
    };
    let [pattern, scrutinee] = condition.children.as_slice() else {
        return Err(
            "Scalar Option if-let requires complete pattern and initializer evidence".into(),
        );
    };

    let option = TypeToken::Option(Box::new(TypeToken::Scalar("usize".into())));
    if condition.expansion.is_some()
        || condition.is_unsafe_block
        || crate::data_aggregates::token(condition)? != "bool"
        || crate::data_aggregates::parsed_type(scrutinee)? != option
    {
        return Err("Scalar Option if-let requires an owned Option<usize> initializer".into());
    }

    let binding = patterns::some_pattern(pattern, "usize", true, facts)?;
    let checked = IfLet {
        source: node,
        condition,
        scrutinee,
        binding,
        then_branch,
        else_branch,
    };
    original(&checked, facts)?;

    Ok(checked)
}

fn original(checked: &IfLet<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let node = checked.source;
    let condition = checked.condition;
    let pattern = &condition.children[0];
    let scrutinee = checked.scrutinee;

    let syntax = crate::source_evidence::original_expression(node, facts)?;
    let syn::Expr::If(branch) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Scalar Option if-let loses original branch syntax".into());
    };
    let syn::Expr::Let(selected) = branch.cond.as_ref() else {
        return Err("Scalar Option if-let does not permit grouped or chained conditions".into());
    };
    if !branch.attrs.is_empty() || !selected.attrs.is_empty() {
        return Err("Scalar Option if-let cannot hide conditional attributes".into());
    }

    original_pattern(&selected.pat)?;
    source::corresponding(&*branch.cond, condition, node, facts)?;
    source::corresponding(&*selected.pat, pattern, node, facts)?;
    source::corresponding(&*selected.expr, scrutinee, node, facts)?;

    source::pattern_syntax(&selected.pat, pattern, node, facts)?;
    source::corresponding(&branch.then_branch, &node.children[1], node, facts)?;
    let (_, otherwise) = branch
        .else_branch
        .as_ref()
        .ok_or("Scalar Option if-let loses else")?;

    source::corresponding(&**otherwise, &node.children[2], node, facts)
}

fn original_pattern(pattern: &syn::Pat) -> Result<(), String> {
    let syn::Pat::TupleStruct(some) = pattern else {
        return Err("Scalar Option if-let requires an original Some payload pattern".into());
    };
    if !some.attrs.is_empty()
        || some.qself.is_some()
        || some
            .path
            .segments
            .iter()
            .any(|segment| !matches!(segment.arguments, syn::PathArguments::None))
        || !matches!(some.elems.first(), Some(syn::Pat::Ident(binding)) if binding.attrs.is_empty())
    {
        return Err("Scalar Option if-let requires a plain unmodified payload pattern".into());
    }
    Ok(())
}

pub(crate) fn sources(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::field_locations::selected_operations(facts, &["expr_if"], candidate)
}
