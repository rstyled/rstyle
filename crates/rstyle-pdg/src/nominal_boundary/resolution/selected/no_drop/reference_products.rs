//! Source-closed cleanup proof for fresh products containing shared byte views.
//! This witness grants no construction, return-origin, Copy or call permission.
use super::product_fields::Proof;
use super::*;
use crate::concrete_types::VariantShape;
use crate::nominal_boundary::resolution::{Resolution, verify_declarations};

pub(in crate::nominal_boundary::resolution::selected) fn supported(tree: &TypeTree) -> bool {
    fields(tree).is_some_and(|fields| {
        fields
            .iter()
            .all(|field| byte_view(&field.ty) || copy_shape(&field.ty, 0))
    })
}

/// Common source shape only; each caller separately restricts field semantics.
pub(in crate::nominal_boundary::resolution) fn fields(
    tree: &TypeTree,
) -> Option<&[concrete_types::Field]> {
    let TypeTree::Struct { source, fields, .. } = tree else {
        return None;
    };
    (concrete_types::lifetimes::declaration_count(source) == Ok(1)
        && !fields.is_empty()
        && fields.len() <= concrete_types::MAX_FIELDS
        && fields.iter().any(|field| byte_view(&field.ty)))
    .then_some(fields.as_slice())
}

fn byte_view(tree: &TypeTree) -> bool {
    matches!(tree, TypeTree::Shared(inner)
        if matches!(inner.as_ref(), TypeTree::Slice(element)
            if matches!(element.as_ref(), TypeTree::Scalar(token) if token == "u8")))
}

fn copy_shape(tree: &TypeTree, depth: usize) -> bool {
    if depth > concrete_types::MAX_DEPTH {
        return false;
    }
    match tree {
        TypeTree::Scalar(token) => concrete_types::scalar(token) && token != "unit",
        TypeTree::Option(payload) => matches!(payload.as_ref(), TypeTree::Scalar(token)
            if concrete_types::scalar(token) && token != "unit"),
        TypeTree::Struct { source, fields, .. } => {
            concrete_types::lifetimes::declaration_count(source) == Ok(0)
                && !fields.is_empty()
                && fields.iter().all(|field| copy_shape(&field.ty, depth + 1))
        }
        TypeTree::MixedEnum { variants, .. } => {
            !variants.is_empty()
                && variants.len() <= concrete_types::MAX_VARIANTS
                && variants.iter().all(|variant| match &variant.shape {
                    VariantShape::Unit {} => true,
                    VariantShape::SingleTuple { field } => copy_shape(&field.ty, depth + 1),
                    _ => false,
                })
        }
        _ => false,
    }
}

/// The caller gates activation; an unregistered proof does not authorize values.
pub(in crate::nominal_boundary::resolution::selected) fn verify(
    facts: &HirFnFacts,
    tree: &TypeTree,
    resolution: &Resolution,
    origins: &Origins,
) -> Result<CheckedNoDrop, String> {
    concrete_types::validate(tree)?;
    if !supported(tree) {
        return Err("Cleanup product is outside the complete byte-view/Copy-field grammar".into());
    }
    let environment = &resolution.environment;
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    crate::declaration_origins::verify_in(&interface, origins, environment)?;
    verify_declarations(resolution, origins, facts)?;
    let checker = Proof::reference(facts, resolution, origins);
    let key = checker.declaration(tree)?;
    let item = checker.item(&key)?;
    let syn::Item::Struct(item) = item else {
        return Err("Cleanup product requires an original struct".into());
    };
    declaration(item, tree)?;
    super::super::structure(facts, item, false)?;
    for field in tree.nominal_fields() {
        if !byte_view(&field.ty) {
            checker.copy(&field.ty)?;
        }
    }
    checker.closure(&key)?;

    let (nominal, _) = tree
        .nominal_parts()
        .ok_or("Missing cleanup product identity")?;
    Ok(CheckedNoDrop {
        nominal: nominal.clone(),
    })
}

fn declaration(item: &syn::ItemStruct, tree: &TypeTree) -> Result<(), String> {
    let syn::Fields::Named(fields) = &item.fields else {
        return Err("Cleanup product requires complete named fields".into());
    };
    if concrete_types::lifetimes::parameter_count(&item.generics)? != 1 {
        return Err("Cleanup product requires one explicit source lifetime".into());
    }
    let Some(syn::GenericParam::Lifetime(parameter)) = item.generics.params.first() else {
        return Err("Missing cleanup product source lifetime".into());
    };
    if fields.named.len() != tree.nominal_fields().count() {
        return Err("Cleanup product original field inventory disagrees".into());
    }

    for (source, observed) in fields.named.iter().zip(tree.nominal_fields()) {
        if source
            .ident
            .as_ref()
            .is_none_or(|name| name != &observed.name)
        {
            return Err("Cleanup product original field order disagrees".into());
        }
        if byte_view(&observed.ty) && !explicit_view(&source.ty, &parameter.lifetime) {
            return Err(
                "Cleanup byte view must use the declared source lifetime explicitly".into(),
            );
        }
    }
    Ok(())
}

fn explicit_view(ty: &syn::Type, lifetime: &syn::Lifetime) -> bool {
    matches!(ty, syn::Type::Reference(reference)
        if reference.mutability.is_none()
            && reference.lifetime.as_ref() == Some(lifetime)
            && matches!(reference.elem.as_ref(), syn::Type::Slice(slice)
                if matches!(slice.elem.as_ref(), syn::Type::Path(path)
                    if path.qself.is_none() && path.path.is_ident("u8"))))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod witness;
