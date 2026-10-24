//! Mixed enum shapes are type evidence for borrowed owners, not enum operations.
use super::*;

pub(super) fn token(
    nominal: &Nominal,
    variants: &[crate::concrete_types::MixedVariant],
) -> TypeToken {
    TypeToken::MixedEnum {
        nominal: nominal.clone(),
        variants: named_tokens(
            variants,
            |variant| &variant.name,
            |variant| variant.shape.map(|field| TypeToken::from_tree(&field.ty)),
        ),
    }
}

pub(super) fn check(
    nominal: &Nominal,
    variants: &[(String, VariantShape<TypeToken>)],
    depth: usize,
    entries: &mut usize,
) -> Result<(), String> {
    let mut names = std::collections::BTreeSet::new();
    if !nominal
        .source_identity
        .as_ref()
        .is_some_and(|id| !id.is_empty())
        || variants.is_empty()
        || variants.len() > crate::concrete_types::MAX_VARIANTS
        || !variants
            .iter()
            .any(|(_, shape)| matches!(shape, VariantShape::Unit {}))
        || !variants.iter().any(|(_, shape)| {
            matches!(
                shape,
                VariantShape::SingleTuple { .. } | VariantShape::Named { .. }
            )
        })
    {
        return Err("Mixed enum requires complete bounded source/unit/payload evidence".into());
    }
    for (name, shape) in variants {
        if name.is_empty() || !names.insert(name) {
            return Err("Mixed enum variant identity is absent or repeated".into());
        }
        if let VariantShape::Named { fields } = shape {
            let mut names = std::collections::BTreeSet::new();
            if fields.is_empty() || fields.len() > crate::concrete_types::MAX_FIELDS
                || fields.iter().any(|(name, value)| name.is_empty() || !names.insert(name)
                    || !matches!(value, TypeToken::Scalar(value) if crate::concrete_types::scalar(value) && value != "unit")) {
                return Err("Named enum requires unique ordered scalar fields".into());
            }
        }
        for field in shape.fields() {
            if !crate::concrete_types::closed_payload(field) {
                return Err("Mixed enum requires closed scalar/struct payloads".into());
            }
            super::check(field, depth + 1, entries)?;
        }
    }
    Ok(())
}

pub(super) fn borrowed_use(ty: &TypeToken) -> bool {
    match ty {
        TypeToken::Shared(inner) if matches!(inner.as_ref(), TypeToken::Struct { .. }) => true,
        TypeToken::MixedEnum { .. } => false,
        TypeToken::Struct { fields, .. } => fields.iter().all(|(_, field)| borrowed_use(field)),
        TypeToken::Tuple(values) => values.iter().all(borrowed_use),
        TypeToken::Result { ok, err } => borrowed_use(ok) && borrowed_use(err),
        TypeToken::Shared(inner)
        | TypeToken::Slice(inner)
        | TypeToken::Array { element: inner, .. }
        | TypeToken::Option(inner) => borrowed_use(inner),
        _ => true,
    }
}

/// Observing a reference-bearing variant never authorizes its tag or payload.
pub(super) fn reference_payload(ty: &TypeToken) -> bool {
    match ty {
        TypeToken::MixedEnum { .. } if crate::data_origins::shared(ty) => true,
        TypeToken::Result { ok, err } => reference_payload(ok) || reference_payload(err),
        TypeToken::Tuple(values) => values.iter().any(reference_payload),
        TypeToken::Struct { fields, .. }
        | TypeToken::Enum {
            variants: fields, ..
        } => fields.iter().any(|(_, field)| reference_payload(field)),
        TypeToken::Shared(value)
        | TypeToken::Slice(value)
        | TypeToken::Array { element: value, .. }
        | TypeToken::Option(value) => reference_payload(value),
        _ => false,
    }
}
