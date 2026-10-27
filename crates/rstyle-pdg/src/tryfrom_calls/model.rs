//! Actual observation schema; strings marked display_only never identify types.
use crate::{ByteSpan, HirRef};
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Definition {
    pub path: String,
    pub definition_identity: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub stable_crate_id: String,
    pub local: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Argument {
    pub kind: String,
    pub value: String,
    pub display_only: bool,
    pub identity_status: String,
    pub identity: Structural,
}

impl Argument {
    pub fn known(&self) -> Result<&Structural, String> {
        if self.kind != "type"
            || self.value.is_empty()
            || !self.display_only
            || self.identity_status != "known"
        {
            return Err("Conversion substitution lacks complete structural identity".into());
        }
        Ok(&self.identity)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Structural {
    Builtin {
        name: String,
    },
    Adt {
        definition: Definition,
        arguments: Vec<Structural>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Signature {
    pub inputs: Vec<Vec<Argument>>,
    pub output: Vec<Argument>,
    pub bound_variables: usize,
    pub abi: String,
    pub safety: String,
    pub variadic: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Header {
    pub is_const: bool,
    pub is_async: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Member {
    pub definition: Definition,
    pub trait_member: Option<Definition>,
    pub source: crate::nominal_boundary::census::expansions::FileSpan,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AssociatedError {
    pub trait_member: Definition,
    pub implementation_member: Definition,
    pub source: crate::nominal_boundary::census::expansions::FileSpan,
    pub declared_type: Vec<Argument>,
    pub normalized_projection: Vec<Argument>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Observation {
    pub identity: String,
    pub admission_authority: bool,
    pub method: Definition,
    pub implementation: Definition,
    pub trait_definition: Definition,
    pub trait_method: Definition,
    pub implementation_source: crate::nominal_boundary::census::expansions::FileSpan,
    pub method_source: crate::nominal_boundary::census::expansions::FileSpan,
    pub method_header: Header,
    pub members: Vec<Member>,
    pub trait_arguments: Vec<Argument>,
    pub selected: Definition,
    pub selected_arguments: Vec<Argument>,
    pub requested_signature: Signature,
    pub selected_signature: Signature,
    pub associated_error: Option<AssociatedError>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Call {
    pub hir: HirRef,
    pub span: ByteSpan,
    pub expanded: bool,
    pub status: String,
    pub requested: Definition,
    pub selected: Definition,
    pub requested_arguments: Vec<Argument>,
    pub selected_arguments: Vec<Argument>,
    pub signature: serde_json::Value,
    #[serde(default)]
    pub compiled_dependency: Option<serde_json::Value>,
}
