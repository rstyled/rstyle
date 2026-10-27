//! Bounded shape vocabulary never supplies source or ownership authority.
use super::TypeToken;

pub(super) fn owner(token: &TypeToken) -> bool {
    crate::data_types::bounded_struct(token, leaf, None)
}

fn leaf(token: &TypeToken) -> bool {
    match token {
        TypeToken::Scalar(value) => value != "unit" && crate::concrete_types::scalar(value),
        TypeToken::Array { element, .. } => {
            matches!(element.as_ref(), TypeToken::Scalar(_)) && leaf(element)
        }
        _ => false,
    }
}

pub(super) fn contains(token: &TypeToken, owner: &TypeToken) -> bool {
    match token {
        TypeToken::Struct { nominal, fields } => {
            matches!(owner, TypeToken::Struct { nominal: expected, .. } if nominal == expected)
                || fields.iter().any(|(_, field)| contains(field, owner))
        }
        TypeToken::ScalarProductInstance { fields, .. }
        | TypeToken::Enum {
            variants: fields, ..
        } => fields.iter().any(|(_, field)| contains(field, owner)),
        TypeToken::MixedEnum { variants, .. } => variants
            .iter()
            .flat_map(|(_, shape)| shape.fields())
            .any(|field| contains(field, owner)),
        TypeToken::Tuple(fields) => fields.iter().any(|field| contains(field, owner)),
        TypeToken::Result { ok, err } => contains(ok, owner) || contains(err, owner),
        TypeToken::Shared(inner)
        | TypeToken::Slice(inner)
        | TypeToken::Option(inner)
        | TypeToken::Array { element: inner, .. }
        | TypeToken::MappedIntConversionResult { ok: inner }
        | TypeToken::NeverConversionResult { ok: inner } => contains(inner, owner),
        TypeToken::Scalar(_)
        | TypeToken::NonZero(_)
        | TypeToken::MutableScalar(_)
        | TypeToken::Str
        | TypeToken::IntConversionResult(_)
        | TypeToken::NeverError
        | TypeToken::RangeUsize
        | TypeToken::RangeToUsize
        | TypeToken::RangeFromUsize
        | TypeToken::SliceArrayResult { .. }
        | TypeToken::FieldlessEnum { .. } => false,
    }
}
