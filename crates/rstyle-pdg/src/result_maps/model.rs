//! Structural compiler observations; rendered type strings carry no authority.
use crate::{ByteSpan, HirRef, generated_sources::capture::Definition};
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Inventory {
    pub identity: String,
    pub complete: bool,
    pub scope: String,
    pub calls: Vec<Call>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Call {
    pub hir: HirRef,
    pub span: ByteSpan,
    pub expanded: bool,
    pub status: String,
    pub requested: Definition,
    pub selected: Definition,
    pub requested_arguments: Vec<Argument>,
    pub selected_arguments: Vec<Argument>,
    pub signature: DisplaySignature,
    pub compiled_dependency: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DisplaySignature {
    pub display_only: bool,
    pub inputs: Vec<String>,
    pub output: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Argument {
    pub kind: String,
    pub value: String,
    pub display_only: bool,
    pub identity_status: String,
    pub identity: Structural,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Structural {
    Builtin {
        name: String,
    },
    Adt {
        definition: Definition,
        arguments: Vec<Structural>,
    },
    TupleConstructor {
        definition: Definition,
        owner: Definition,
        arguments: Vec<Structural>,
        signature: ConstructorSignature,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConstructorSignature {
    pub input: Box<Structural>,
    pub output: Box<Structural>,
    pub bound_variables: usize,
    pub abi: String,
    pub safety: String,
    pub variadic: bool,
}
