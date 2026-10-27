//! Opaque conversion observations never grant standalone or nested value use.
use super::TypeToken;

pub(super) fn contains(token: &TypeToken) -> bool {
    match token {
        TypeToken::MappedIntConversionResult { .. }
        | TypeToken::NeverConversionResult { .. }
        | TypeToken::NeverError => true,
        TypeToken::Tuple(values) => values.iter().any(contains),
        TypeToken::Array { element, .. }
        | TypeToken::Shared(element)
        | TypeToken::Slice(element)
        | TypeToken::Option(element) => contains(element),
        TypeToken::Result { ok, err } => contains(ok) || contains(err),
        TypeToken::Struct { fields, .. } | TypeToken::ScalarProductInstance { fields, .. } => {
            fields.iter().any(|(_, value)| contains(value))
        }
        TypeToken::Enum { variants, .. } => variants.iter().any(|(_, value)| contains(value)),
        TypeToken::MixedEnum { variants, .. } => variants
            .iter()
            .any(|(_, shape)| shape.fields().any(contains)),
        _ => false,
    }
}
