//! Original compiler graph observations for the existing advisory proposer.
use crate::facts::Pdg;
use serde::{Deserialize, Serialize};

pub const KEY: &str = "rstyle.compiler-pattern-graph/1";
pub const CAPTURE_ENV: &str = "RSTYLE_COMPILER_PATTERNS";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub schema: String,
    pub owner: String,
    pub graph: Pdg,
}

impl Observation {
    pub fn read(raw: &serde_json::Value) -> Result<Self, String> {
        let nodes = raw["graph"]["nodes"]
            .as_array()
            .ok_or("Missing compiler pattern nodes")?;
        for node in nodes {
            super::require_fields(
                node,
                &[
                    "ty",
                    "callee_id",
                    "callee_sig",
                    "macro_name",
                    "lit_kind",
                    "detail",
                    "line",
                ],
            )?;
            super::require_signature(&node["callee_sig"])?;
        }
        for edge in raw["graph"]["edges"]
            .as_array()
            .ok_or("Missing compiler pattern edges")?
        {
            super::require_fields(edge, &["arg_pos"])?;
        }

        let observation = Self::deserialize(raw).map_err(|error| error.to_string())?;
        if observation.schema != KEY || observation.owner.is_empty() {
            return Err("Unknown compiler pattern schema or missing owner".into());
        }
        if observation
            .graph
            .edges
            .iter()
            .any(|edge| edge.from >= nodes.len() || edge.to >= nodes.len())
        {
            return Err("Compiler pattern edge has no node".into());
        }
        Ok(observation)
    }
}

#[cfg(test)]
mod tests;
