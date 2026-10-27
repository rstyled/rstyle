//! One payload classification shared by captured and serialized instance types.
use super::Argument;
use crate::{
    concrete_types::TypeTree,
    data_types::TypeToken,
    nonzero_values::{Observation, Scalar},
};

pub(crate) enum Payload<'a> {
    Scalar(&'a str),
    NonZero(&'a Observation),
    Other,
}

pub(crate) trait ProductPayload {
    fn payload(&self) -> Payload<'_>;
}

impl ProductPayload for TypeTree {
    fn payload(&self) -> Payload<'_> {
        match self {
            Self::Scalar(value) => Payload::Scalar(value),
            Self::NonZero(value) => Payload::NonZero(value),
            _ => Payload::Other,
        }
    }
}

impl ProductPayload for TypeToken {
    fn payload(&self) -> Payload<'_> {
        match self {
            Self::Scalar(value) => Payload::Scalar(value),
            Self::NonZero(value) => Payload::NonZero(value),
            _ => Payload::Other,
        }
    }
}

impl Argument {
    pub(crate) fn matches_payload(self, value: &impl ProductPayload) -> bool {
        match (self, value.payload()) {
            (Self::U8 | Self::U16, Payload::Scalar(value)) => value == self.as_str(),
            (Self::NonZeroU8, Payload::NonZero(value)) => Self::valid_nonzero(value, Scalar::U8),
            (Self::NonZeroU16, Payload::NonZero(value)) => Self::valid_nonzero(value, Scalar::U16),
            _ => false,
        }
    }

    fn valid_nonzero(value: &Observation, scalar: Scalar) -> bool {
        value.scalar == scalar && value.check_shape().is_ok()
    }
}
