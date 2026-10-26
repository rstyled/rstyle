//! Bind original tuple syntax to every compiler binding and ordered scalar slot.
use super::*;
use std::collections::BTreeSet;

pub(super) fn verify<'a>(
    node: &'a HirNode,
    pattern: &'a HirNode,
    initializer: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Vec<Entry<'a>>, String> {
    let local = declaration(node, facts)?;
    let syn::Pat::Tuple(tuple) = &local.pat else {
        return Err("Tuple binding source pattern is not a flat tuple".into());
    };
    let init = local
        .init
        .as_ref()
        .ok_or("Tuple binding source lacks initializer")?;
    if !local.attrs.is_empty()
        || init.diverge.is_some()
        || tuple.elems.len() != pattern.children.len()
    {
        return Err("Tuple binding source changes its complete local shape".into());
    }

    plain(node, facts)?;
    plain(pattern, facts)?;
    crate::source_evidence::verify_child_source(&local.pat, pattern, node, facts)?;
    crate::source_evidence::verify_child_source(init.expr.as_ref(), initializer, node, facts)?;

    let mut seen = BTreeSet::new();
    pattern
        .children
        .iter()
        .zip(&tuple.elems)
        .enumerate()
        .map(|(position, (child, syntax))| {
            let entry = entry(child, position, facts)?;
            let syn::Pat::Ident(ident) = syntax else {
                return Err("Tuple binding requires only named scalar identifiers".into());
            };
            if !ident.attrs.is_empty()
                || ident.by_ref.is_some()
                || ident.mutability.is_some()
                || ident.subpat.is_some()
                || ident.ident != entry.binding.name
                || !seen.insert(&entry.binding.symbol)
            {
                return Err("Tuple binding repeats, rebinds or mutates a scalar component".into());
            }
            crate::source_evidence::verify_child_source(syntax, child, node, facts)?;
            Ok(entry)
        })
        .collect()
}

fn declaration(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<syn::Local, String> {
    let bytes = crate::source_evidence::byte_range(&facts.source.bytes, node.span)?;
    let syn::Stmt::Local(local) = syn::parse_str::<syn::Stmt>(bytes).map_err(|e| e.to_string())?
    else {
        return Err("Tuple binding requires its original local declaration".into());
    };
    Ok(local)
}

fn entry<'a>(
    pattern: &'a HirNode,
    position: usize,
    facts: &'a dyn EvidenceFacts,
) -> Result<Entry<'a>, String> {
    plain(pattern, facts)?;
    let matches: Vec<_> = facts
        .bindings
        .iter()
        .filter(|binding| binding.hir == pattern.hir)
        .collect();
    let [binding] = matches.as_slice() else {
        return Err("Tuple component requires one exact compiler binding".into());
    };

    let token = crate::data_aggregates::token(pattern)?;
    if pattern.kind != "pat_binding"
        || pattern.binding.as_ref() != Some(&binding.symbol)
        || pattern.binding_mut != Some(false)
        || pattern.param_index.is_some()
        || !pattern.children.is_empty()
        || binding.is_param
        || binding.is_mut
        || binding.decl_span != pattern.span
        || binding.ty.normalized.as_deref() != Some(token)
        || !crate::concrete_types::scalar(token)
        || token == "unit"
    {
        return Err("Tuple component differs from its immutable concrete scalar local".into());
    }
    Ok(Entry {
        pattern,
        binding,
        position,
        token,
    })
}

fn plain(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if node.expansion.is_some()
        || node.is_unsafe_block
        || node.res.is_some()
        || node.dispatch.is_some()
        || node.lit.is_some()
        || node.hir.krate != 0
        || node.hir.owner != facts.body.hir.owner
        || crate::source_evidence::byte_range(&facts.source.bytes, node.span)? != node.snippet
    {
        return Err("Tuple binding loses its original owned source/HIR identity".into());
    }
    Ok(())
}
