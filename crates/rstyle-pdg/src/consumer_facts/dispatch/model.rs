use super::super::Availability;
use crate::ByteSpan;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Assessment<T = Candidate> {
    pub availability: Availability<Option<T>>,
    pub reason: Option<String>,
    pub budget_exhausted: bool,
}

impl<T> Assessment<T> {
    pub(super) fn negative(reason: &str) -> Self {
        Self {
            availability: Availability::Known(None),
            reason: Some(reason.into()),
            budget_exhausted: false,
        }
    }

    pub(super) fn unsupported(reason: &str) -> Self {
        Self {
            availability: Availability::Unsupported {
                reason: reason.into(),
            },
            reason: None,
            budget_exhausted: false,
        }
    }

    pub(super) fn unknown(reason: &str) -> Self {
        Self {
            availability: Availability::Unknown {
                reason: reason.into(),
            },
            reason: None,
            budget_exhausted: false,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Candidate {
    pub enum_name: String,
    pub caller_header: ByteSpan,
    pub enum_header: ByteSpan,
    pub signature: Signature,
    pub arms: Vec<Arm>,
    pub advisory: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Signature {
    pub shared_self: bool,
    pub ordinary_parameters: usize,
    pub return_type: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Arm {
    pub variant: String,
    pub arm_span: ByteSpan,
    pub receiver_type: String,
    pub method: String,
    pub callee_header: ByteSpan,
    pub origin: MethodOrigin,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MethodOrigin {
    Inherent,
    LocalTrait {
        trait_name: String,
        member: String,
        trait_header: ByteSpan,
    },
}
