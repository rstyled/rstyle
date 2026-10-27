//! Bounded simultaneous assignment to distinct scalar lexical bindings.
use crate::{HirNode, unit_store::EvidenceFacts};
use std::collections::BTreeSet;

pub(crate) fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .supports("rust.scalar-tuple-assignment")
    {
        return Err("Tuple assignment requires its registered capability".into());
    }
    let [left, right] = node.children.as_slice() else {
        return Err("Tuple assignment lacks target and value tuples".into());
    };
    if left.kind != "expr_tup"
        || right.kind != "expr_tup"
        || left.children.is_empty()
        || left.children.len() != right.children.len()
    {
        return Err("Tuple assignment requires equally sized flat literal tuples".into());
    }
    let syn::Expr::Assign(syntax) = crate::source_evidence::original_expression(node, facts)?
    else {
        return Err("Missing original assignment syntax".into());
    };
    let (syn::Expr::Tuple(targets), syn::Expr::Tuple(values)) = (&*syntax.left, &*syntax.right)
    else {
        return Err("Original assignment is not a pair of literal tuples".into());
    };
    if targets.elems.len() != left.children.len() || values.elems.len() != right.children.len() {
        return Err("Tuple assignment loses source elements".into());
    }

    let mut seen = BTreeSet::new();
    for ((target, value), syntax) in left
        .children
        .iter()
        .zip(&right.children)
        .zip(&targets.elems)
    {
        let symbol = verify_target(target, value, syntax, facts)?;
        if !seen.insert(symbol) {
            return Err("Repeated tuple assignment target".into());
        }
    }
    Ok(())
}

fn verify_target<'a>(
    target: &HirNode,
    value: &HirNode,
    syntax: &syn::Expr,
    facts: &'a dyn EvidenceFacts,
) -> Result<&'a str, String> {
    let syn::Expr::Path(path) = syntax else {
        return Err("Nonlocal tuple target".into());
    };
    let binding = target
        .res
        .as_ref()
        .filter(|res| res.kind == "local")
        .and_then(|res| res.target.as_ref())
        .and_then(|hir| facts.bindings.iter().find(|binding| &binding.hir == hir))
        .ok_or("Unresolved tuple assignment target")?;
    let token = crate::data_aggregates::token(target)?;

    if target.kind != "expr_path"
        || !binding.is_mut
        || path.qself.is_some()
        || !path.path.is_ident(&binding.name)
        || crate::data_aggregates::token(value)? != token
        || binding.ty.normalized.as_deref() != Some(token)
        || token == "unit"
        || !crate::concrete_types::scalar(token)
    {
        return Err(
            "Tuple target must be a distinct mutable scalar local with matching RHS".into(),
        );
    }
    Ok(&binding.symbol)
}
