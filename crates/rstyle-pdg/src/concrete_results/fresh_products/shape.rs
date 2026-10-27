//! Shape vocabulary only; actual source, Copy and cleanup are separate proofs.
use super::*;

pub(crate) const GRAMMAR: crate::data_types::ResultGrammar = crate::data_types::ResultGrammar {
    success,
    error: copy_errors::error,
};

pub(super) fn success(token: &TypeToken) -> bool {
    crate::data_types::bounded_struct(
        token,
        field_shape,
        Some(crate::data_types::shared_byte_slice),
    )
}

fn field_shape(token: &TypeToken) -> bool {
    crate::data_types::shared_byte_slice(token)
        || scalar(token)
        || matches!(token, TypeToken::Option(payload) if scalar(payload))
}

fn scalar(token: &TypeToken) -> bool {
    matches!(token, TypeToken::Scalar(value) if value != "unit" && crate::concrete_types::scalar(value))
}

pub(crate) fn used(token: &TypeToken) -> bool {
    Search::ProductResult.matches(token)
}

pub(super) fn contains_owned(token: &TypeToken, product: &TypeToken) -> bool {
    Search::Owned(product).matches(token)
}

#[derive(Clone, Copy)]
enum Search<'a> {
    ProductResult,
    Owned(&'a TypeToken),
}

impl Search<'_> {
    fn matches(self, token: &TypeToken) -> bool {
        let found = match self {
            Self::ProductResult => {
                matches!(token, TypeToken::Result { ok, err } if GRAMMAR.permits(ok, err))
            }
            Self::Owned(product) => matches!(token, TypeToken::Result { .. }) || token == product,
        };
        found || self.children(token)
    }

    fn children(self, token: &TypeToken) -> bool {
        match token {
            TypeToken::Result { ok, err } => self.matches(ok) || self.matches(err),
            TypeToken::Tuple(values) => values.iter().any(|value| self.matches(value)),
            TypeToken::Struct { fields, .. }
            | TypeToken::Enum {
                variants: fields, ..
            } => fields.iter().any(|(_, field)| self.matches(field)),
            TypeToken::MixedEnum { variants, .. } => variants
                .iter()
                .flat_map(|(_, shape)| shape.fields())
                .any(|field| self.matches(field)),
            TypeToken::Shared(inner)
            | TypeToken::Slice(inner)
            | TypeToken::Option(inner)
            | TypeToken::Array { element: inner, .. } => self.matches(inner),
            _ => false,
        }
    }
}
