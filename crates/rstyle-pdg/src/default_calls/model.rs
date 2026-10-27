//! Closed concrete Default observations; display strings never identify types.
use crate::{ByteSpan, HirRef, concrete_types::Nominal};
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Definition {
    pub path: String,
    pub definition_identity: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub stable_crate_id: String,
    pub local: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Argument {
    kind: String,
    value: String,
    display_only: bool,
    identity_status: String,
    identity: Structural,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Structural {
    Adt {
        definition: Definition,
        arguments: Vec<serde_json::Value>,
    },
}

pub(super) fn nominal(arguments: &[Argument], nominal: &Nominal) -> Result<Definition, String> {
    let [argument] = arguments else {
        return Err("Default requires exactly one complete concrete Self type".into());
    };
    let Structural::Adt {
        definition,
        arguments,
    } = &argument.identity;
    let path = definition
        .path
        .strip_prefix(&definition.crate_name)
        .and_then(|tail| tail.strip_prefix("::"))
        .unwrap_or(&definition.path);

    if argument.kind != "type"
        || argument.value.is_empty()
        || !argument.display_only
        || argument.identity_status != "known"
        || !arguments.is_empty()
        || !definition.local
        || definition.crate_name.is_empty()
        || definition.definition_identity.is_empty()
        || definition.stable_crate_id.is_empty()
        || path != nominal.definition_path
    {
        return Err("Default structural Self differs from its complete local nominal owner".into());
    }
    Ok(definition.clone())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Signature {
    inputs: Vec<Vec<Argument>>,
    output: Vec<Argument>,
    bound_variables: usize,
    abi: String,
    safety: String,
    variadic: bool,
}

impl Signature {
    pub fn verify(&self, owner: &Nominal) -> Result<Definition, String> {
        if !self.inputs.is_empty()
            || self.bound_variables != 0
            || self.abi != "\"Rust\""
            || self.safety != "Safe"
            || self.variadic
        {
            return Err("Default requires its safe ordinary zero-argument Rust signature".into());
        }
        nominal(&self.output, owner)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Header {
    pub is_const: bool,
    pub is_async: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Member {
    pub definition: Definition,
    pub trait_member: Option<Definition>,
    pub source: crate::nominal_boundary::census::expansions::FileSpan,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Observation {
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
    pub associated_error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Selection {
    pub identity: String,
    pub admission_authority: bool,
    pub hir: HirRef,
    pub span: ByteSpan,
    pub requested: Definition,
    pub trait_definition: Definition,
    pub requested_arguments: Vec<Argument>,
    pub selected: Definition,
    pub implementation: Definition,
    pub selected_arguments: Vec<Argument>,
    pub requested_signature: Signature,
    pub selected_signature: Signature,
}
