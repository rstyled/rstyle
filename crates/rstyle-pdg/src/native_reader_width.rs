//! Closed native-reader widths; observed tuples alone confer no source authority.
use crate::{data_types::TypeToken, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.named-error-u16-reader-flow";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Width {
    U16,
    U32,
}

impl Width {
    pub(crate) fn for_result(facts: &dyn EvidenceFacts) -> Result<Self, String> {
        let token = crate::data_types::parse(
            facts
                .signature
                .return_ty
                .normalized
                .as_deref()
                .ok_or("Native reader requires its complete concrete Result")?,
        )?;

        let TypeToken::Result { ok, .. } = token else {
            return Err("Native reader requires a Result boundary".into());
        };
        let width = Self::for_success(&ok)
            .ok_or("Native reader requires an exact supported scalar/count tuple")?;

        if width == Self::U16 && !crate::extraction_profile::facts_support(facts, CAPABILITY) {
            return Err("Native u16 reader requires its registered policy".into());
        }
        Ok(width)
    }

    pub(crate) fn for_success(token: &TypeToken) -> Option<Self> {
        let TypeToken::Tuple(values) = token else {
            return None;
        };
        let [TypeToken::Scalar(value), TypeToken::Scalar(count)] = values.as_slice() else {
            return None;
        };
        if count != "usize" {
            return None;
        }
        match value.as_str() {
            "u16" => Some(Self::U16),
            "u32" => Some(Self::U32),
            _ => None,
        }
    }

    pub(crate) fn scalar(self) -> &'static str {
        match self {
            Self::U16 => "u16",
            Self::U32 => "u32",
        }
    }

    pub(crate) fn bytes(self) -> usize {
        match self {
            Self::U16 => 2,
            Self::U32 => 4,
        }
    }

    pub(crate) fn native(self) -> crate::data_bytes::Kind {
        match self {
            Self::U16 => crate::data_bytes::Kind::NativeU16,
            Self::U32 => crate::data_bytes::Kind::Native,
        }
    }
}

pub(crate) fn u16_success(token: &TypeToken, facts: &dyn EvidenceFacts) -> bool {
    Width::for_success(token) == Some(Width::U16)
        && crate::extraction_profile::facts_support(facts, CAPABILITY)
}
