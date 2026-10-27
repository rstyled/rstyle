//! Raw receipts bind compiler observations to the original node and child order.
use super::{Adjustment, BindingMode, IDENTITY, RESOLUTION, TypeShape};
use crate::frontend::{ByteSpan, HirNode, HirRef, PathRes};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "reason", deny_unknown_fields)]
pub enum CaptureStatus {
    Captured,
    Unsupported(String),
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub canonical: HirRef,
    pub name: String,
    pub identifier_span: ByteSpan,
    pub written_mode: BindingMode,
    pub written_mutable: bool,
    pub effective_mode: Option<BindingMode>,
    pub effective_mutable: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Details {
    Slice {
        prefix: Vec<HirRef>,
        rest: Option<HirRef>,
        suffix: Vec<HirRef>,
    },
    Binding {
        binding: Binding,
        subpattern: Option<HirRef>,
    },
    Range {
        start: Option<HirRef>,
        end: Option<HirRef>,
        inclusive: bool,
    },
    ByteLiteral {
        value: u8,
        negated: bool,
    },
    Path {
        resolution: PathRes,
    },
    Rest,
    Unsupported {
        compiler_kind: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub identity: String,
    pub hir: HirRef,
    pub span: ByteSpan,
    pub status: CaptureStatus,
    pub ty: TypeShape,
    pub adjustments: Vec<Adjustment>,
    pub details: Details,
}

impl Observation {
    pub fn resolution(&self) -> Result<PathRes, serde_json::Error> {
        PathRes::observation(RESOLUTION, self)
    }

    pub fn decode(node: &HirNode) -> Result<Self, String> {
        let resolution = node
            .res
            .as_ref()
            .ok_or("Missing slice-pattern observation")?;
        if resolution.kind != RESOLUTION || resolution.target.is_some() {
            return Err("Invalid slice-pattern observation resolution".into());
        }

        let observation: Self = serde_json::from_str(&resolution.description)
            .map_err(|error| format!("Invalid slice-pattern observation: {error}"))?;
        if observation.identity != IDENTITY
            || observation.hir != node.hir
            || observation.span != node.span
        {
            return Err("Slice-pattern observation belongs to a different node".into());
        }

        Ok(observation)
    }
}
