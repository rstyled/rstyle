//! Closed observations retain unsupported types and adjustment kinds explicitly.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", deny_unknown_fields)]
pub enum TypeShape {
    Missing,
    Byte,
    Slice(Box<TypeShape>),
    Shared(Box<TypeShape>),
    Mutable(Box<TypeShape>),
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Adjustment {
    pub kind: AdjustmentKind,
    pub source: TypeShape,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdjustmentKind {
    BuiltinDeref,
    OverloadedDeref,
    PinDeref,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingMode {
    Value,
    Shared,
    Mutable,
    PinnedShared,
    PinnedMutable,
}
