//! Original closure syntax and complete parameter/body correspondence.

use crate::{HirNode, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

pub(super) fn invocation(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;
    let syn::Expr::MethodCall(call) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Immediate map_or loses original method syntax".into());
    };
    if call.method != "map_or"
        || call.args.len() != 2
        || call.turbofish.is_some()
        || !call.attrs.is_empty()
    {
        return Err("Immediate map_or requires two original arguments without substitution".into());
    }

    for (syntax, child) in std::iter::once(&*call.receiver)
        .chain(call.args.iter())
        .zip(&node.children)
    {
        corresponding(syntax, child, node, facts)?;
    }
    Ok(())
}

pub(super) fn verify(
    node: &HirNode,
    binding: &HirNode,
    body: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;
    let syn::Expr::Closure(closure) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Callback lacks original closure syntax".into());
    };
    if !closure.attrs.is_empty()
        || closure.lifetimes.is_some()
        || closure.constness.is_some()
        || closure.movability.is_some()
        || closure.asyncness.is_some()
        || closure.capture.is_some()
        || !matches!(closure.output, syn::ReturnType::Default)
    {
        return Err("Callback modifiers and callable annotations remain unsupported".into());
    }

    if closure.inputs.len() != 1 {
        return Err("Callback requires exactly one original parameter".into());
    }
    let Some(syn::Pat::Ident(parameter)) = closure.inputs.first() else {
        return Err("Callback requires one original name parameter".into());
    };
    if parameter.by_ref.is_some()
        || parameter.mutability.is_some()
        || parameter.subpat.is_some()
        || !parameter.attrs.is_empty()
    {
        return Err("Callback parameter requires a plain immutable binding".into());
    }

    corresponding(parameter, binding, node, facts)?;
    corresponding(&*closure.body, body, node, facts)?;
    comparison(&closure.body, body, node, facts)
}

fn comparison(
    syntax: &syn::Expr,
    body: &HirNode,
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let syn::Expr::Binary(binary) = crate::data_aggregates::ungroup(syntax) else {
        return Err("Callback requires original equality syntax".into());
    };
    if !matches!(binary.op, syn::BinOp::Eq(_)) || !binary.attrs.is_empty() {
        return Err("Callback requires unmodified equality syntax".into());
    }
    for (syntax, child) in [&*binary.left, &*binary.right]
        .into_iter()
        .zip(&body.children)
    {
        let syn::Expr::Path(path) = crate::data_aggregates::ungroup(syntax) else {
            return Err("Callback equality requires plain binding reads".into());
        };
        if path.qself.is_some() || path.path.get_ident().is_none() || !path.attrs.is_empty() {
            return Err("Callback equality requires unqualified binding reads".into());
        }
        corresponding(syntax, child, node, facts)?;
    }
    Ok(())
}

fn corresponding(
    syntax: &impl Spanned,
    child: &HirNode,
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let bytes = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Callback source lies outside its owning function")?;
    let expected = crate::ByteSpan {
        start: node.span.start
            + crate::source_evidence::byte_offset(bytes, syntax.span().start()) as u32,
        end: node.span.start
            + crate::source_evidence::byte_offset(bytes, syntax.span().end()) as u32,
    };
    if !crate::source_evidence::corresponding_span(&facts.source.bytes, expected, child.span) {
        return Err("Callback loses original source/body correspondence".into());
    }
    Ok(())
}
