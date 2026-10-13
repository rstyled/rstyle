//! Observational method receiver evidence. This is not a call admission registry.

use crate::{ByteSpan, HirRef, concrete_types::Observation};
use serde::{Deserialize, Serialize};

pub const KEY: &str = "data_method_receivers";
pub const IDENTITY: &str = "rstyle.rust.method-receiver-capture/1";
pub const MAX_RECORDS: usize = 4096;
pub const MAX_ADJUSTMENTS: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    pub identity: String,
    pub methods: Vec<Method>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Method {
    pub call: HirRef,
    pub source_span: ByteSpan,
    pub receiver: HirRef,
    pub receiver_type: Observation,
    pub adjusted_type: Observation,
    pub adjustments: Vec<Adjustment>,
    pub signature_inputs: Vec<Observation>,
    pub signature_result: Observation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Adjustment {
    pub kind: Kind,
    pub target: Observation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "reason",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Kind {
    BuiltinDereference,
    SharedBorrow,
    MutableBorrow,
    Unsupported(String),
}
