//! Complete original scalar error enums; this proof never grants Copy.
use super::*;
use crate::concrete_types::VariantShape;

pub(in crate::nominal_boundary::resolution::selected) const CAPABILITY: &str =
    "rust.checked-borrowed-result-flow";

/// Bounded implicit-tag enums with complete scalar named/unit alternatives.
pub(in crate::nominal_boundary::resolution::selected) fn supported(tree: &TypeTree) -> bool {
    let TypeTree::MixedEnum { variants, .. } = tree else {
        return false;
    };
    !variants.is_empty()
        && variants.len() <= concrete_types::MAX_VARIANTS
        && variants.iter().all(|variant| match &variant.shape {
            VariantShape::Unit {} => true,
            VariantShape::Named { fields } => {
                !fields.is_empty()
                    && fields.len() <= concrete_types::MAX_FIELDS
                    && fields
                        .iter()
                        .all(|(_, field)| scalar_field(&field.ty).is_some())
            }
            _ => false,
        })
}

fn scalar_field(tree: &TypeTree) -> Option<&str> {
    match tree {
        TypeTree::Scalar(token) if concrete_types::scalar(token) && token != "unit" => Some(token),
        _ => None,
    }
}

pub(super) fn declaration(item: &syn::ItemEnum, tree: &TypeTree) -> Result<(), String> {
    if !supported(tree) || !item.generics.params.is_empty() || item.generics.where_clause.is_some()
    {
        return Err("Cleanup enum requires its complete bounded scalar error family".into());
    }
    let TypeTree::MixedEnum { variants, .. } = tree else {
        unreachable!()
    };
    if item.variants.len() != variants.len() {
        return Err("Cleanup enum variant inventory disagrees with original source".into());
    }

    for (position, (source, observed)) in item.variants.iter().zip(variants).enumerate() {
        if observed.position != position
            || source.ident != observed.name
            || source.discriminant.is_some()
        {
            return Err("Cleanup enum variant name/order or discriminant disagrees".into());
        }
        variant(&source.fields, &observed.shape)?;
    }
    Ok(())
}

fn variant(
    source: &syn::Fields,
    observed: &VariantShape<concrete_types::Field>,
) -> Result<(), String> {
    match (source, observed) {
        (syn::Fields::Unit, VariantShape::Unit {}) => Ok(()),
        (syn::Fields::Named(source), VariantShape::Named { fields })
            if source.named.len() == fields.len() =>
        {
            for (position, (source, (name, observed))) in
                source.named.iter().zip(fields).enumerate()
            {
                let token = scalar_field(&observed.ty)
                    .ok_or("Cleanup enum payload is not an integer/bool scalar")?;
                if source.ident.as_ref().is_none_or(|ident| ident != name)
                    || observed.name != *name
                    || observed.position != position
                    || observed.declaration.as_str() != token
                    || !matches!(&source.ty, syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident(token))
                {
                    return Err("Cleanup enum named scalar field source disagrees".into());
                }
            }
            Ok(())
        }
        _ => Err("Cleanup enum variant field shape disagrees".into()),
    }
}

#[cfg(test)]
mod tests;
