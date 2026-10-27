//! Source-closed no-drop proof for fresh reference-free scalar/array structs.
//! This grants no Copy, construction, movement, storage or call permission.
use super::product_fields::Proof;
use super::*;
use crate::nominal_boundary::resolution::{Resolution, verify_declarations};

pub(in crate::nominal_boundary::resolution) fn supported(tree: &TypeTree) -> bool {
    let TypeTree::Struct { source, fields, .. } = tree else {
        return false;
    };
    !fields.is_empty()
        && fields.len() <= concrete_types::MAX_FIELDS
        && syn::parse_str::<syn::ItemStruct>(source).is_ok_and(|item| {
            item.generics.params.is_empty()
                && item.generics.where_clause.is_none()
                && matches!(item.fields, syn::Fields::Named(_))
        })
        && fields.iter().all(|field| scalar_field(&field.ty))
}

fn scalar_field(tree: &TypeTree) -> bool {
    match tree {
        TypeTree::Scalar(token) => concrete_types::scalar(token) && token != "unit",
        TypeTree::Array { element, length } => {
            *length <= concrete_types::MAX_FIELDS as u64
                && matches!(element.as_ref(), TypeTree::Scalar(token)
                    if concrete_types::scalar(token) && token != "unit")
        }
        _ => false,
    }
}

/// Consumes resolved source evidence only; value eligibility is proved separately.
pub(in crate::nominal_boundary::resolution) fn verify_source(
    facts: &HirFnFacts,
    tree: &TypeTree,
    resolution: &Resolution,
    origins: &Origins,
) -> Result<CheckedNoDrop, String> {
    concrete_types::validate(tree)?;
    if !supported(tree) {
        return Err("Fresh cleanup product requires nongeneric named scalar/array fields".into());
    }

    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    crate::declaration_origins::verify_in(
        &interface,
        origins,
        &resolution.environment,
        &facts.build.frontend_options,
    )?;
    // Includes exact source field types and independently resolved array lengths.
    verify_declarations(resolution, origins, facts)?;
    let checker = Proof::fresh_scalar_product(facts, resolution, origins);
    let key = checker.declaration(tree)?;

    let syn::Item::Struct(item) = checker.item(&key)? else {
        return Err("Fresh cleanup product requires an original struct".into());
    };

    declaration(item, tree)?;
    super::super::structure(facts, item, false)?;
    checker.closure(&key)?;
    let (nominal, _) = tree
        .nominal_parts()
        .ok_or("Missing fresh product identity")?;
    Ok(CheckedNoDrop {
        nominal: nominal.clone(),
    })
}

fn declaration(item: &syn::ItemStruct, tree: &TypeTree) -> Result<(), String> {
    let syn::Fields::Named(fields) = &item.fields else {
        return Err("Fresh cleanup product requires complete named fields".into());
    };
    if !item.generics.params.is_empty() || item.generics.where_clause.is_some() {
        return Err("Fresh cleanup product requires a nongeneric original declaration".into());
    }
    if fields.named.len() != tree.nominal_fields().count()
        || fields
            .named
            .iter()
            .zip(tree.nominal_fields())
            .any(|(source, observed)| {
                source
                    .ident
                    .as_ref()
                    .is_none_or(|name| name != &observed.name)
            })
    {
        return Err("Fresh cleanup product original field inventory disagrees".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
