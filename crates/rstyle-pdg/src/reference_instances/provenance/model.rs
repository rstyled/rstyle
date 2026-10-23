//! Strict bounded wire grammar. Unsupported symbolic forms fail during decoding.
use crate::ByteSpan;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Observation {
    pub identity: String,
    pub complete: bool,
    pub definition: Definition,
    pub arguments: Vec<Argument>,
    pub owners: Vec<Owner>,
    pub signature: Signature,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Definition {
    pub path: String,
    pub definition_identity: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub stable_crate_id: String,
    pub local: bool,
}

impl Definition {
    pub(super) fn same_crate(&self, other: &Self) -> bool {
        self.local
            && other.local
            && self.crate_name == other.crate_name
            && self.stable_crate_id == other.stable_crate_id
            && !self.definition_identity.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Anchor {
    pub source_file: String,
    pub span: ByteSpan,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Owner {
    pub definition: Definition,
    #[serde(deserialize_with = "required_parent")]
    pub parent: Option<Definition>,
    pub declaration: Anchor,
    pub header: Vec<Anchor>,
    pub parameters: Vec<Parameter>,
    pub source_predicates: Vec<Anchor>,
    pub clauses: Vec<Clause>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Parameter {
    pub definition: Definition,
    pub index: u32,
    pub name: String,
    pub kind: String,
    pub declaration: Anchor,
    pub name_anchor: Anchor,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Clause {
    pub anchor: Anchor,
    pub kind: String,
    // Parsed and verified by the shared reviewed compiler authority.
    pub trait_definition: CoreDefinition,
    pub positive: bool,
    pub original_arguments: Vec<SymbolicArgument>,
    pub substituted_arguments: Vec<Argument>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CoreDefinition {
    pub path: String,
    pub definition_identity: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub stable_crate_id: String,
    pub local: bool,
    pub crate_hash: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Signature {
    pub anchor: Anchor,
    pub inputs: Vec<Symbolic>,
    pub output: Symbolic,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Symbolic {
    Parameter {
        index: u32,
        name: String,
    },
    Builtin {
        name: String,
    },
    Slice {
        element: Box<Symbolic>,
    },
    Reference {
        lifetime: Region,
        mutable: bool,
        referent: Box<Symbolic>,
    },
    Adt {
        definition: Definition,
        arguments: Vec<SymbolicArgument>,
    },
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Region {
    Parameter { index: u32, name: String },
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(super) enum SymbolicArgument {
    Type(Symbolic),
    Lifetime(Region),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Argument {
    pub kind: String,
    #[serde(rename = "value")]
    pub _display_value: String,
    pub display_only: bool,
    pub identity_status: String,
    pub identity: Structural,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Structural {
    Builtin { name: String },
    Str,
    Slice { element: Box<Structural> },
    ErasedRegion,
}

fn required_parent<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Definition>, D::Error> {
    Option::<Definition>::deserialize(deserializer)
}
