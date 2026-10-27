//! Small structural constructor observations; display strings are not type identities.
use super::*;
use crate::{ByteSpan, HirRef, frontend::PathRes, generated_sources::capture::Definition};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Callee {
    pub hir: HirRef,
    pub span: ByteSpan,
    pub snippet: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    pub source_file: String,
    pub span: ByteSpan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenericArgument {
    ErasedLifetime,
    Scalar(super::super::Argument),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueType {
    SharedByteSlice,
    Scalar(super::super::Argument),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub position: u32,
    pub definition: Definition,
    pub ty: ValueType,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signature {
    pub inputs: [ValueType; 2],
    pub output_owner: Definition,
    pub output_arguments: [GenericArgument; 2],
    pub bound_variables: usize,
    pub abi: String,
    pub safety: String,
    pub variadic: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub identity: String,
    pub hir: HirRef,
    pub span: ByteSpan,
    pub callee: Callee,
    pub constructor: Definition,
    pub owner: Definition,
    pub declaration: Declaration,
    pub arguments: [GenericArgument; 2],
    pub fields: [Field; 2],
    pub values: [HirRef; 2],
    pub signature: Signature,
}

impl Observation {
    pub fn resolution(&self) -> Result<PathRes, serde_json::Error> {
        PathRes::observation(RESOLUTION, self)
    }

    pub fn decode(node: &HirNode) -> Result<Self, String> {
        let resolution = node
            .res
            .as_ref()
            .ok_or("Unknown scalar product constructor selection")?;
        if resolution.kind != RESOLUTION || resolution.target.is_some() {
            return Err("Unsupported scalar product constructor resolution".into());
        }
        let observation: Self = serde_json::from_str(&resolution.description)
            .map_err(|error| format!("Invalid scalar product constructor observation: {error}"))?;

        if observation.identity != IDENTITY
            || observation.hir != node.hir
            || observation.span != node.span
        {
            return Err("Scalar product constructor observation belongs to another node".into());
        }
        Ok(observation)
    }
}
