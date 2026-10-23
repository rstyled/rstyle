//! One source tail call, one argument read and the complete original node roster.
use super::{
    super::body::model::{Argument, Observation, Type},
    Result,
};
use crate::reference_instances::view_types::{BorrowedViewType, ViewType};
use crate::{HirFnFacts, HirNode};
use syn::spanned::Spanned;

pub(super) fn verify(facts: &HirFnFacts) -> Result<&HirNode> {
    let syntax: syn::ItemFn =
        syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;
    let [syn::Stmt::Expr(syn::Expr::Call(call), None)] = syntax.block.stmts.as_slice() else {
        return Err("From forwarder must return its sole complete constructor call".into());
    };
    let syn::Expr::Path(target) = call.func.as_ref() else {
        return Err("From forwarder callee must be its original direct path".into());
    };
    let Some(syn::Expr::Path(read)) = call.args.first() else {
        return Err("From forwarder must pass its one original formal directly".into());
    };

    if call.args.len() != 1
        || !call.attrs.is_empty()
        || !target.attrs.is_empty()
        || !read.attrs.is_empty()
        || target.qself.is_some()
        || target.path.leading_colon.is_some()
        || target.path.segments.len() != 2
        || target.path.segments[0].ident != "Input"
        || target.path.segments[1].ident != "new"
        || target
            .path
            .segments
            .iter()
            .any(|segment| !matches!(segment.arguments, syn::PathArguments::None))
        || read.qself.is_some()
        || !read.path.is_ident("haystack")
    {
        return Err("From forwarder changes its exact source call/argument path".into());
    }
    verify_roster(facts, &syntax, call, read)
}

fn verify_roster<'a>(
    facts: &'a HirFnFacts,
    syntax: &syn::ItemFn,
    call: &syn::ExprCall,
    read: &syn::ExprPath,
) -> Result<&'a HirNode> {
    let [formal] = facts.signature.params.as_slice() else {
        return Err("From forwarder must have its sole original formal".into());
    };
    let [_binding] = facts.bindings.as_slice() else {
        return Err("From forwarder binding census omits or invents storage".into());
    };

    node(facts, &facts.body, &syntax.block, "expr_block")?;
    let invocation = &facts.body.children[0];
    node(facts, invocation, call, "expr_call")?;
    node(facts, &invocation.children[0], read, "expr_path")?;

    let Some(syn::FnArg::Typed(input)) = syntax.sig.inputs.first() else {
        return Err("From forwarder lacks its original typed formal".into());
    };
    let syn::Pat::Ident(pattern) = input.pat.as_ref() else {
        return Err("From forwarder formal is not one plain binding".into());
    };
    if pattern.ident != "haystack"
        || pattern.by_ref.is_some()
        || pattern.mutability.is_some()
        || pattern.subpat.is_some()
        || !pattern.attrs.is_empty()
    {
        return Err("From forwarder changes its plain formal binding".into());
    }

    node(facts, &formal.pat, pattern, "pat_binding")?;
    formal_type(facts, input)?;
    initialized_formal(facts, invocation)?;
    Ok(invocation)
}

fn initialized_formal(facts: &HirFnFacts, invocation: &HirNode) -> Result<()> {
    let formal = &facts.signature.params[0];
    let binding = &facts.bindings[0];
    if binding.hir != formal.pat.hir
        || binding.symbol != formal.symbol
        || formal.pat.binding.as_ref() != Some(&binding.symbol)
        || binding.name != "haystack"
        || !binding.is_param
        || binding.is_mut
        || binding.decl_span != formal.name_span
        || formal.pat.ty.as_ref() != Some(&binding.ty)
        || !binding.ty.spelling.is_empty()
        || binding.scope != format!("scope:source:fn:0:{}", facts.source.bytes.len())
        || !invocation.children[0]
            .res
            .as_ref()
            .is_some_and(|resolution| {
                resolution.kind == "local" && resolution.target.as_ref() == Some(&formal.pat.hir)
            })
    {
        return Err("From forwarder loses its exact initialized formal origin".into());
    }
    Ok(())
}

fn formal_type(facts: &HirFnFacts, syntax: &syn::PatType) -> Result<()> {
    let formal = &facts.signature.params[0];
    let span = crate::source_evidence::original_span(&facts.source.bytes, syntax.ty.span());
    let spelling = crate::source_evidence::byte_range(&facts.source.bytes, span)?;

    let expected = &formal.ty;
    let read = &facts.body.children[0].children[0];
    if formal.position != 0
        || formal.ty_span != span
        || expected.spelling != spelling
        || expected.normalized.is_some()
        || expected.observed != "&H"
        || read.ty.as_ref() != formal.pat.ty.as_ref()
        || formal.pat.ty.as_ref().is_none_or(|ty| {
            ty.normalized.is_some() || ty.observed != "&H" || !ty.spelling.is_empty()
        })
    {
        return Err(
            "From formal type spelling/observations differ from original source roles".into(),
        );
    }
    Ok(())
}

fn node(facts: &HirFnFacts, node: &HirNode, syntax: &impl Spanned, kind: &str) -> Result<()> {
    let span = crate::source_evidence::original_span(&facts.source.bytes, syntax.span());
    let children = usize::from(matches!(kind, "expr_block" | "expr_call"));
    let parameter = kind == "pat_binding";
    let local = kind == "expr_path";

    if node.kind != kind
        || node.children.len() != children
        || node.span != span
        || facts
            .source
            .bytes
            .get(span.start as usize..span.end as usize)
            != Some(node.snippet.as_str())
        || node.hir.krate != 0
        || node.hir.owner != facts.body.hir.owner
        || node.ty.is_none()
        || node.expansion.is_some()
        || node.is_unsafe_block
        || node.dispatch.is_some()
        || node.lit.is_some()
        || node.param_index != parameter.then_some(0)
        || node.binding.is_some() != parameter
        || node.binding_mut != parameter.then_some(false)
        || node.res.is_some() != local
    {
        return Err(
            "From forwarder has missing, extra or contradictory original HIR evidence".into(),
        );
    }
    Ok(())
}

pub(super) fn projection(
    record: &Observation,
    constructor: &Observation,
    call: &HirNode,
    haystack: &str,
) -> Result<()> {
    if record.arguments.len() != constructor.arguments.len()
        || !record
            .arguments
            .iter()
            .zip(&constructor.arguments)
            .all(|(a, b)| equal(a, b))
        || record.projected_result.len() != 1
        || constructor.projected_result.len() != 1
        || !equal(
            &record.projected_result[0],
            &constructor.projected_result[0],
        )
    {
        return Err(
            "From forwarder does not preserve the complete constructor instance/result".into(),
        );
    }

    let facts = &record.original;
    let input = Type::haystack(haystack)?.shared();
    let output = &constructor.projected_result[0].identity;
    let expected = [
        (&facts.signature.params[0].pat, &input),
        (&facts.body, output),
        (call, output),
        (&call.children[0], &input),
    ];

    if record.nodes.len() != expected.len() {
        return Err("From forwarder projected node census is incomplete".into());
    }
    for (node, (original, ty)) in record.nodes.iter().zip(expected) {
        let [value] = node.ty.as_slice() else {
            return Err("From node lacks its exact projected type".into());
        };
        if node.kind != "value" || node.hir != original.hir || !typed(value, "type", ty) {
            return Err("From projected node loses its exact identity/type/order".into());
        }
    }
    Ok(())
}

pub(super) fn equal(a: &Argument, b: &Argument) -> bool {
    typed(a, &b.kind, &b.identity) && b.display_only && b.identity_status == "known"
}

pub(super) fn typed(value: &Argument, kind: &str, ty: &Type) -> bool {
    value.kind == kind
        && value.display_only
        && value.identity_status == "known"
        && value.identity == *ty
}
