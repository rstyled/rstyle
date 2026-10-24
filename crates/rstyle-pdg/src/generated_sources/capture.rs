//! Raw compiler observations; source ownership must be independently established.
use crate::{HirRef, nominal_boundary::census::expansions::FileSpan};
use serde::{Deserialize, Serialize};

pub const KEY: &str = "rstyle.rust.generated-source";
pub const IDENTITY: &str = "rstyle.rust.generated-source-observation/1";
pub const UNIT_IDENTITY: &str = "rstyle.rust.generated-source-unit/1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub path: String,
    pub definition_identity: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub stable_crate_id: String,
    pub local: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub raw: FileSpan,
    pub marks: Vec<Mark>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mark {
    pub expansion: String,
    pub transparency: Transparency,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transparency {
    Opaque,
    SemiOpaque,
    Transparent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expansion {
    pub expansion: String,
    pub parent: Option<String>,
    pub kind: ExpansionKind,
    pub name: String,
    pub macro_definition: Option<Definition>,
    pub definition: Option<FileSpan>,
    pub invocation: Origin,
    pub edition: String,
    pub local_inner_macros: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpansionKind {
    Bang,
    Attr,
    Derive,
    Desugaring,
    AstPass,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeOrigin {
    pub path: Vec<u32>,
    pub hir: HirRef,
    pub kind: String,
    pub origin: Origin,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Emitted {
    pub definition: Definition,
    pub parent: Definition,
    pub kind: String,
    pub origin: Origin,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub identity: String,
    pub owner: Definition,
    pub implementation: Definition,
    pub defining_expansion: String,
    pub signature: Origin,
    pub body: Origin,
    pub nodes: Vec<NodeOrigin>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Outcome {
    Observed { observation: Box<Observation> },
    Unsupported { owner: Definition, reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitCapture {
    pub identity: String,
    pub outcomes: Vec<Outcome>,
    pub expansions: Vec<Expansion>,
    pub emitted: Vec<Emitted>,
    pub complete: bool,
    pub scope: String,
}
