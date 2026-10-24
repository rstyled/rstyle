//! One closed payload grammar for source trees and canonical value tokens.
use crate::{concrete_types::TypeTree, data_types::TypeToken};

pub(crate) trait Payload: Sized {
    fn scalar(&self) -> Option<&str>;
    fn shared_str(&self) -> bool;
    fn fields(&self) -> Option<impl Iterator<Item = &Self>>;
}

pub(crate) fn closed_payload(value: &impl Payload) -> bool {
    if let Some(scalar) = value.scalar() {
        return super::scalar(scalar) && scalar != "unit";
    }
    if value.shared_str() {
        return true;
    }
    value
        .fields()
        .is_some_and(|mut fields| fields.all(closed_payload))
}

impl Payload for TypeTree {
    fn shared_str(&self) -> bool {
        matches!(self, Self::Shared(referent) if referent.as_ref() == &Self::Str)
    }

    fn scalar(&self) -> Option<&str> {
        match self {
            Self::Scalar(value) => Some(value),
            _ => None,
        }
    }

    fn fields(&self) -> Option<impl Iterator<Item = &Self>> {
        match self {
            Self::Struct { fields, .. } => Some(fields.iter().map(|field| &field.ty)),
            _ => None,
        }
    }
}

impl Payload for TypeToken {
    fn shared_str(&self) -> bool {
        matches!(self, Self::Shared(referent) if referent.as_ref() == &Self::Str)
    }

    fn scalar(&self) -> Option<&str> {
        match self {
            Self::Scalar(value) => Some(value),
            _ => None,
        }
    }

    fn fields(&self) -> Option<impl Iterator<Item = &Self>> {
        match self {
            Self::Struct { fields, .. } => Some(fields.iter().map(|(_, field)| field)),
            _ => None,
        }
    }
}
