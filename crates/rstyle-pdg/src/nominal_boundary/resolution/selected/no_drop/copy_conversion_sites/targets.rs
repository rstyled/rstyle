use super::*;
use crate::concrete_types::{VariantShape, source_proof::PathResolution};
use crate::nominal_boundary::resolution::Scoped;

pub(super) fn endpoints(interface: &InterfaceObservations) -> Result<Endpoints, String> {
    let [Observation::Captured { ty: input }] = interface.parameters.as_slice() else {
        return Err("Copy conversion requires one complete observed input".into());
    };
    let Observation::Captured { ty: output } = &interface.result else {
        return Err("Copy conversion requires a complete observed output".into());
    };

    let direction = match output {
        TypeTree::MixedEnum { .. } => Direction::Wrap,
        TypeTree::Result { .. } => Direction::Unwrap,
        _ => return Err("Copy conversion requires tagged wrapping or inverse Result".into()),
    };
    let endpoints = Endpoints {
        direction,
        input: input.clone(),
        output: output.clone(),
    };
    vocabulary(&endpoints)?;
    Ok(endpoints)
}

pub(super) fn vocabulary(endpoints: &Endpoints) -> Result<(), String> {
    let TypeTree::MixedEnum { variants, .. } = endpoints.tagged() else {
        return Err("Copy conversion requires its concrete tagged enum".into());
    };
    let [unit, payload] = variants.as_slice() else {
        return Err("Copy conversion requires both complete enum alternatives".into());
    };
    if unit.position != 0
        || payload.position != 1
        || unit.name == payload.name
        || !matches!(unit.shape, VariantShape::Unit {})
        || !matches!(&payload.shape, VariantShape::SingleTuple { field }
            if field.position == 0 && field.name == "0" && &field.ty == endpoints.product())
    {
        return Err("Copy conversion enum tag or exact payload identity differs".into());
    }

    let TypeTree::Struct { fields, .. } = endpoints.product() else {
        return Err("Copy conversion payload must be the original static product".into());
    };
    if fields.len() != 2
        || fields[0].position != 0
        || fields[1].position != 1
        || !matches!(&fields[0].ty, TypeTree::Shared(inner) if inner.as_ref() == &TypeTree::Str)
        || fields[1].ty != TypeTree::Scalar("bool".into())
        || endpoints.error().is_some_and(
            |error| !matches!(error, TypeTree::Struct { fields, .. } if fields.is_empty()),
        )
    {
        return Err("Copy conversion product or nominal unit error lies outside contract".into());
    }
    Ok(())
}

pub(super) fn verify(
    facts: &HirFnFacts,
    resolution: &Resolution,
    origins: &Origins,
    endpoints: &Endpoints,
) -> Result<(), String> {
    let proof = product_fields::Proof::copy_product(facts, resolution, origins);

    for tree in [
        Some(endpoints.product()),
        Some(endpoints.tagged()),
        endpoints.error(),
    ]
    .into_iter()
    .flatten()
    {
        let key = proof.declaration(tree)?;
        let item = proof.item(&key)?;
        copy_components::verify_target_source(item, tree)?;
        original_payload(item, resolution, &key, endpoints)?;
        let attributes = super::super::super::declaration_attributes(item, tree)?;
        if !super::super::super::checked_copy(facts, origins, tree, attributes)? {
            return Err(
                "Copy conversion target lacks its exact positive builtin Copy proof".into(),
            );
        }
        match item {
            syn::Item::Struct(item) => super::super::super::structure(facts, item, true)?,
            syn::Item::Enum(item) => super::super::super::scalar_enum(facts, item)?,
            _ => {
                return Err("Copy conversion target is not an original nominal declaration".into());
            }
        }
    }
    Ok(())
}

fn original_payload(
    item: &syn::Item,
    resolution: &Resolution,
    key: &[String],
    endpoints: &Endpoints,
) -> Result<(), String> {
    match item {
        syn::Item::Enum(item) => {
            let Some(field) = item
                .variants
                .iter()
                .nth(1)
                .and_then(|variant| variant.fields.iter().next())
            else {
                return Err("Copy conversion lacks its original tagged payload annotation".into());
            };
            let syn::Type::Path(path) = &field.ty else {
                return Err(
                    "Copy conversion tagged payload requires its original nominal path".into(),
                );
            };
            let namespace = key[..key.len().saturating_sub(1)].to_vec();
            let scope = Scoped {
                resolution,
                namespace: &namespace,
            };
            if scope.resolve(path)? != *endpoints.product() {
                return Err("Copy conversion original tagged payload identity differs".into());
            }
        }
        syn::Item::Struct(item) if error_identity(endpoints, key) => {
            nominal_unit(item)?;
        }
        _ => {}
    }
    Ok(())
}

fn error_identity(endpoints: &Endpoints, key: &[String]) -> bool {
    endpoints.error().is_some_and(|error| {
        error.nominal_parts().is_some_and(|(nominal, _)| {
            nominal
                .definition_path
                .split("::")
                .eq(key.iter().map(String::as_str))
        })
    })
}

pub(super) fn nominal_unit(item: &syn::ItemStruct) -> Result<(), String> {
    if !matches!(item.fields, syn::Fields::Unit) {
        return Err("Copy conversion error must be its original nominal unit struct".into());
    }
    Ok(())
}
