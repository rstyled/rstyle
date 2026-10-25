//! One saved shared formal, one exact prefix arm, and an exhaustive None fallback.
use super::*;
use syn::spanned::Spanned;

pub(super) fn verify<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<(&'a HirNode, &'a HirNode, &'a HirNode), String> {
    let [scrutinee, success, failure] = node.children.as_slice() else {
        return Err("Prefix semantics require exactly two original match arms".into());
    };
    if node.kind != "expr_match"
        || node.expansion.is_some()
        || success.kind != "match_arm"
        || failure.kind != "match_arm"
        || success.children.len() != 2
        || failure.children.len() != 2
        || success.expansion.is_some()
        || failure.expansion.is_some()
    {
        return Err("Prefix semantics require an original unguarded exhaustive match".into());
    }
    let [formal] = facts.signature.params.as_slice() else {
        return Err("Prefix semantics require exactly one saved slice formal".into());
    };
    if crate::source_evidence::resolved_local_leaf(scrutinee) != Some(&formal.pat.hir)
        || formal.position != 0
        || formal.pat.binding_mut != Some(false)
        || crate::data_aggregates::parsed_type(scrutinee)?
            != crate::reference_instances::shared_bytes()
        || formal.ty.normalized.as_deref() != Some(crate::data_aggregates::token(scrutinee)?)
    {
        return Err("Prefix semantics must preserve the exact immutable input formal".into());
    }

    source(node, facts)?;
    formal_source(scrutinee, facts)?;
    wildcard(&failure.children[0])?;
    none(&failure.children[1], facts)?;
    Ok((scrutinee, &success.children[1], &failure.children[1]))
}

fn formal_source(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let function: syn::ItemFn =
        syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;
    let Some(syn::FnArg::Typed(parameter)) = function.sig.inputs.first() else {
        return Err("Prefix source lacks its ordinary input formal".into());
    };
    let syn::Pat::Ident(binding) = parameter.pat.as_ref() else {
        return Err("Prefix input formal must be an immutable identifier".into());
    };
    let syntax = crate::source_evidence::original_expression(node, facts)?;

    if function.sig.inputs.len() != 1
        || !binding.attrs.is_empty()
        || binding.by_ref.is_some()
        || binding.mutability.is_some()
        || binding.subpat.is_some()
        || crate::source_evidence::original_span(&facts.source.bytes, binding.span())
            != facts.signature.params[0].pat.span
        || !matches!(syntax, syn::Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none()
            && crate::source_evidence::plain_relative_path(&path.path) == Some(vec![binding.ident.to_string()]))
    {
        return Err("Prefix scrutinee must save its original formal exactly once".into());
    }
    Ok(())
}

fn source(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;
    let syn::Expr::Match(syntax) = syntax else {
        return Err("Missing original prefix match".into());
    };
    if !syntax.attrs.is_empty()
        || syntax.arms.len() != 2
        || syntax
            .arms
            .iter()
            .any(|arm| !arm.attrs.is_empty() || arm.guard.is_some())
        || !matches!(&syntax.arms[1].pat, syn::Pat::Wild(wildcard) if wildcard.attrs.is_empty())
    {
        return Err("Prefix source must retain its complete unguarded wildcard complement".into());
    }

    crate::source_evidence::verify_child_span(node, &node.children[0], syntax.expr.span(), facts)?;
    for (syntax, arm) in syntax.arms.iter().zip(&node.children[1..]) {
        crate::source_evidence::verify_child_span(
            node,
            &arm.children[0],
            syntax.pat.span(),
            facts,
        )?;
        crate::source_evidence::verify_child_span(
            node,
            &arm.children[1],
            syntax.body.span(),
            facts,
        )?;
    }
    Ok(())
}

fn wildcard(node: &HirNode) -> Result<(), String> {
    if node.kind != "pat_wild"
        || !node.children.is_empty()
        || node.expansion.is_some()
        || node.binding.is_some()
        || node.binding_mut.is_some()
        || node.res.is_some()
    {
        return Err("Prefix failure requires its original payload-free wildcard".into());
    }
    Ok(())
}

fn none(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let resolution = node.res.as_ref().ok_or("Missing original None identity")?;
    let callee: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    let registry =
        crate::data_library::registry_for(facts).ok_or("Unpinned prefix None identity")?;
    let syntax = crate::source_evidence::original_expression(node, facts)?;

    if node.kind != "expr_path"
        || !node.children.is_empty()
        || node.expansion.is_some()
        || node.dispatch.is_some()
        || resolution.kind != "resolved_option_variant"
        || resolution.target.is_some()
        || callee.path.replace("core::", "std::") != "std::prelude::v1::None"
        || callee.crate_name != "core"
        || callee.crate_hash != registry.core_hash
        || registry.definitions.get("std::prelude::v1::None") != Some(&callee.definition_hash)
        || callee.implementation.is_some()
        || !matches!(syntax, syn::Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none()
            && crate::source_evidence::plain_relative_path(&path.path) == Some(vec!["None".into()]))
    {
        return Err("Prefix failure must retain pinned original None with no payload".into());
    }
    Ok(())
}
