//! Exact return roles keep fresh ownership out of ordinary value operations.
use super::{EvidenceFacts, TypeToken, shape};
use crate::HirNode;

pub(super) fn verify(facts: &dyn EvidenceFacts, owner: &TypeToken) -> Result<(), String> {
    for ty in facts
        .signature
        .params
        .iter()
        .map(|parameter| &parameter.ty)
        .chain(facts.bindings.iter().map(|binding| &binding.ty))
    {
        reject_type(ty.normalized.as_deref(), owner)?;
    }
    let source: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;
    crate::source_evidence::closed_body(&source.block)?;
    returning(&facts.body, owner, facts)
}

fn reject_type(token: Option<&str>, owner: &TypeToken) -> Result<(), String> {
    let Some(text) = token else {
        return Ok(());
    };
    let parsed = crate::data_types::parse(text)?;
    if shape::contains(&parsed, owner) {
        return Err("Fresh scalar owner cannot be stored, borrowed, reused or discarded".into());
    }
    Ok(())
}

fn returning(node: &HirNode, owner: &TypeToken, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if node.kind == "expr_ret" {
        let [value] = node.children.as_slice() else {
            return Err("Fresh owner return requires exactly one complete operand".into());
        };
        return returning(value, owner, facts);
    }
    crate::data_aggregates::verify_node_type(
        node,
        owner,
        "Fresh owner return differs from its exact source-checked type",
    )?;
    match node.kind.as_str() {
        "expr_block" => block(node, owner, facts),
        "expr_struct" => constructor(node, owner, facts),
        _ => Err("Fresh scalar owner requires a direct original complete struct return".into()),
    }
}

fn block(node: &HirNode, owner: &TypeToken, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let Some((tail, prefix)) = node.children.split_last() else {
        return Err("Fresh owner block lacks its complete return value".into());
    };
    for statement in prefix {
        ordinary(statement, owner)?;
    }
    returning(tail, owner, facts)
}

fn constructor(node: &HirNode, owner: &TypeToken, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if node.expansion.is_some() || node.dispatch.is_some() {
        return Err("Fresh owner construction must retain original unexpanded source".into());
    }

    let syn::Expr::Struct(source) = crate::source_evidence::original_expression(node, facts)?
    else {
        return Err("Fresh owner requires its original named struct expression".into());
    };
    if source.rest.is_some() {
        return Err("Fresh owner construction cannot consume a struct update base".into());
    }

    crate::data_aggregates::verify(node, facts)?;
    for child in &node.children {
        ordinary(child, owner)?;
    }
    Ok(())
}

fn ordinary(node: &HirNode, owner: &TypeToken) -> Result<(), String> {
    reject_type(
        node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()),
        owner,
    )?;
    for child in &node.children {
        ordinary(child, owner)?;
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn rejects(token: &TypeToken, owner: &TypeToken) -> bool {
    reject_type(Some(&token.text().expect("test token")), owner).is_err()
}
