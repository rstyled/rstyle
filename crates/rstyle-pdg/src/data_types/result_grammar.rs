//! Structural success/error grammar; this grants no ownership or source authority.
use super::{Nominal, TypeToken};

pub(crate) struct ResultGrammar {
    pub success: fn(&TypeToken) -> bool,
    pub error: fn(&TypeToken) -> Option<&Nominal>,
}

impl ResultGrammar {
    pub fn permits(&self, ok: &TypeToken, err: &TypeToken) -> bool {
        (self.success)(ok) && (self.error)(err).is_some()
    }

    /// Find this grammar even inside unsupported storage and borrowed shapes.
    pub fn contains(&self, token: &TypeToken) -> bool {
        match token {
            TypeToken::Result { ok, err } => {
                self.permits(ok, err) || self.contains(ok) || self.contains(err)
            }
            TypeToken::Tuple(values) => values.iter().any(|value| self.contains(value)),
            TypeToken::Struct { fields, .. }
            | TypeToken::Enum {
                variants: fields, ..
            } => fields.iter().any(|(_, value)| self.contains(value)),
            TypeToken::MixedEnum { variants, .. } => variants
                .iter()
                .flat_map(|(_, shape)| shape.fields())
                .any(|value| self.contains(value)),
            TypeToken::Shared(value)
            | TypeToken::Slice(value)
            | TypeToken::Option(value)
            | TypeToken::Array { element: value, .. } => self.contains(value),
            _ => false,
        }
    }
}
