//! Closed diagnostic value-node grammar; displays never establish type identity.
use crate::{HirFnFacts, HirRef};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::reference_instances) struct Observation {
    pub request: Request,
    pub status: String,
    pub definition: Definition,
    pub provenance: Value,
    pub arguments: Vec<Argument>,
    pub original: HirFnFacts,
    pub projected_result: Vec<Argument>,
    pub nodes: Vec<Node>,
    pub conversions: Vec<Value>,
    pub calls: Vec<Value>,
    #[serde(default)]
    pub outer_trait_association: Option<Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::reference_instances) struct Request {
    pub function: String,
    pub haystack: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::reference_instances) struct Definition {
    pub path: String,
    pub definition_identity: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub stable_crate_id: String,
    pub local: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::reference_instances) struct Node {
    pub hir: HirRef,
    pub kind: String,
    pub ty: Vec<Argument>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::reference_instances) struct Argument {
    pub kind: String,
    #[serde(rename = "value")]
    pub _display: String,
    pub display_only: bool,
    pub identity_status: String,
    pub identity: Type,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(in crate::reference_instances) enum Type {
    Builtin {
        name: String,
    },
    Str,
    Slice {
        element: Box<Type>,
    },
    Reference {
        region: String,
        mutable: bool,
        referent: Box<Type>,
    },
    Adt {
        definition: Definition,
        arguments: Vec<Type>,
    },
    ErasedRegion,
}
