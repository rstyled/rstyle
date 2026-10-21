//! A wildcard applied to a plain Copy local place performs no value read or drop.
use crate::concrete_types::source_proof::PathResolution;
use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::data_enums::child_candidate(node, facts, "stmt_let", |pattern| {
        pattern.kind == "pat_wild"
    })
}

pub(crate) fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let [pattern, value] = node.children.as_slice() else {
        return Err("Copy discard requires only a wildcard and a local place".into());
    };
    let source = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original Copy discard source")?;
    let local: syn::Stmt = syn::parse_str(source).map_err(|error| error.to_string())?;
    let syn::Stmt::Local(local) = local else {
        return Err("Discard requires original local syntax".into());
    };
    let initializer = local.init.as_ref().ok_or("Missing discard initializer")?;
    let syn::Expr::Path(path) = initializer.expr.as_ref() else {
        return Err("Discard supports only a plain local place".into());
    };
    if !candidate(node, facts)
        || !matches!(local.pat, syn::Pat::Wild(_))
        || !local.attrs.is_empty()
        || initializer.diverge.is_some()
        || path.qself.is_some()
        || path.path.get_ident().is_none()
        || value.kind != "expr_path"
        || node.expansion.is_some()
        || pattern.expansion.is_some()
        || value.expansion.is_some()
    {
        return Err("Discard loses its original wildcard/local-place shape".into());
    }

    verify_span(node, source, local.pat.span(), pattern)?;
    verify_span(node, source, initializer.expr.span(), value)?;

    let resolution = value
        .res
        .as_ref()
        .ok_or("Missing discarded local resolution")?;
    let binding = facts
        .bindings
        .iter()
        .find(|binding| Some(&binding.hir) == resolution.target.as_ref())
        .ok_or("Discard lacks the exact local binding")?;
    if resolution.kind != "local"
        || path.path.get_ident().unwrap() != &binding.name
        || binding.ty.normalized.as_deref() != Some(crate::data_aggregates::token(value)?)
    {
        return Err("Discard local identity/type disagrees".into());
    }
    verify_copy(value, facts)
}

fn verify_copy(value: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let ty = crate::data_types::parse(crate::data_aggregates::token(value)?)?;
    if matches!(ty, TypeToken::Scalar(_))
        || crate::extraction_profile::facts_support(facts, "rust.accelerator-packing")
            && matches!(ty, TypeToken::Shared(_))
    {
        return Ok(());
    }

    let nominal = match &ty {
        TypeToken::Struct { nominal, .. } | TypeToken::Enum { nominal, .. } => nominal,
        _ => return Err("Discard requires a scalar or checked Copy nominal local".into()),
    };
    let path: syn::TypePath = syn::parse_str(&format!("crate::{}", nominal.definition_path))
        .map_err(|error| error.to_string())?;

    let resolution = facts.nominal_resolution()?;
    let tree = resolution.resolve(&path)?;
    if !resolution.checked_owned_copy(&tree) || TypeToken::from_tree(&tree) != ty {
        return Err("Discard requires exact positive Copy evidence".into());
    }
    Ok(())
}

fn verify_span(
    parent: &HirNode,
    source: &str,
    span: proc_macro2::Span,
    child: &HirNode,
) -> Result<(), String> {
    let expected = crate::ByteSpan {
        start: parent.span.start + crate::source_evidence::byte_offset(source, span.start()) as u32,
        end: parent.span.start + crate::source_evidence::byte_offset(source, span.end()) as u32,
    };
    if child.span != expected {
        return Err("Discard wildcard/place source correspondence disagrees".into());
    }
    Ok(())
}
