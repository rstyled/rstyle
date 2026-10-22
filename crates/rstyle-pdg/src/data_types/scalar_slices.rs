//! Shared scalar-slice shape across captured types and canonical graph tokens.
use super::{TypeToken, TypeTree};

pub(crate) trait SharedScalarSlice {
    fn shared_slice_scalar(&self) -> Option<&str>;
}

impl SharedScalarSlice for TypeToken {
    fn shared_slice_scalar(&self) -> Option<&str> {
        match self {
            Self::Shared(inner) => match inner.as_ref() {
                Self::Slice(element) => match element.as_ref() {
                    Self::Scalar(value) => Some(value),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }
}

impl SharedScalarSlice for TypeTree {
    fn shared_slice_scalar(&self) -> Option<&str> {
        match self {
            Self::Shared(inner) => match inner.as_ref() {
                Self::Slice(element) => match element.as_ref() {
                    Self::Scalar(value) => Some(value),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }
}

pub(crate) fn shared_byte_slice<T: SharedScalarSlice>(value: &T) -> bool {
    value.shared_slice_scalar() == Some("u8")
}
