//! Bounded observed Result shapes; ownership and reference origins are separate proofs.
use crate::{
    concrete_types::{Nominal, TypeTree, VariantShape},
    data_types::{TypeToken, shared_byte_slice},
};

#[cfg(test)]
mod tests;

pub(crate) const CAPABILITY: &str = "rust.checked-borrowed-result-flow";

pub(crate) const GRAMMAR: crate::data_types::ResultGrammar =
    crate::data_types::ResultGrammar { success, error };

pub(crate) fn tree_result(ok: &TypeTree, err: &TypeTree) -> bool {
    let success = shared_byte_slice(ok)
        || matches!(ok, TypeTree::Tuple(values) if matches!(values.as_slice(), [TypeTree::Scalar(value), slice] if value == "usize" && shared_byte_slice(slice)));
    success
        && matches!(err, TypeTree::MixedEnum { variants, .. } if !variants.is_empty()
        && variants.len() <= crate::concrete_types::MAX_VARIANTS
        && variants.iter().all(|variant| match &variant.shape {
            VariantShape::Unit {} => true,
            VariantShape::Named { fields } => !fields.is_empty()
                && fields.len() <= crate::concrete_types::MAX_FIELDS
                && fields.iter().all(|(_, field)| matches!(&field.ty, TypeTree::Scalar(value) if crate::concrete_types::scalar(value) && value != "unit")),
            _ => false,
        }))
}

pub(crate) fn success(token: &TypeToken) -> bool {
    if shared_byte_slice(token) {
        return true;
    }
    matches!(token, TypeToken::Tuple(values) if matches!(values.as_slice(), [TypeToken::Scalar(value), slice] if value == "usize" && shared_byte_slice(slice)))
}

/// This structural predicate deliberately grants neither Copy nor no-drop.
pub(crate) fn error(token: &TypeToken) -> Option<&Nominal> {
    let TypeToken::MixedEnum { nominal, variants } = token else {
        return None;
    };
    if variants.is_empty() || variants.len() > crate::concrete_types::MAX_VARIANTS {
        return None;
    }
    variants.iter().all(|(_, shape)| match shape {
        VariantShape::Unit {} => true,
        VariantShape::Named { fields } => !fields.is_empty()
            && fields.len() <= crate::concrete_types::MAX_FIELDS
            && fields.iter().all(|(_, value)| matches!(value, TypeToken::Scalar(value) if crate::concrete_types::scalar(value) && value != "unit")),
        _ => false,
    }).then_some(nominal)
}

pub(crate) fn owned_use(token: &TypeToken) -> bool {
    match token {
        TypeToken::Result { ok, err } => {
            GRAMMAR.permits(ok, err)
                || crate::concrete_results::copy_errors::GRAMMAR.permits(ok, err)
        }
        TypeToken::MixedEnum { .. } => error(token).is_some(),
        TypeToken::Tuple(values) => values.iter().all(owned_use),
        TypeToken::Struct { fields, .. } => fields.iter().all(|(_, ty)| owned_use(ty)),
        TypeToken::Shared(inner)
        | TypeToken::Slice(inner)
        | TypeToken::Option(inner)
        | TypeToken::Array { element: inner, .. } => !contains_owned(inner),
        _ => true,
    }
}

fn contains_owned(token: &TypeToken) -> bool {
    any(token, &|token| {
        matches!(
            token,
            TypeToken::Result { .. } | TypeToken::MixedEnum { .. }
        )
    })
}

pub(crate) fn uses_result(token: &TypeToken) -> bool {
    any(
        token,
        &|token| matches!(token, TypeToken::Result { ok, err } if GRAMMAR.permits(ok, err)),
    )
}

fn any(token: &TypeToken, predicate: &impl Fn(&TypeToken) -> bool) -> bool {
    if predicate(token) {
        return true;
    }
    match token {
        TypeToken::Result { ok, err } => any(ok, predicate) || any(err, predicate),
        TypeToken::Tuple(values) => values.iter().any(|ty| any(ty, predicate)),
        TypeToken::Struct { fields, .. }
        | TypeToken::Enum {
            variants: fields, ..
        } => fields.iter().any(|(_, ty)| any(ty, predicate)),
        TypeToken::MixedEnum { variants, .. } => variants
            .iter()
            .flat_map(|(_, shape)| shape.fields())
            .any(|ty| any(ty, predicate)),
        TypeToken::Shared(inner)
        | TypeToken::Slice(inner)
        | TypeToken::Option(inner)
        | TypeToken::Array { element: inner, .. } => any(inner, predicate),
        _ => false,
    }
}
