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
        require_graph(&raw["graph"])?;
        let observation = Self::deserialize(raw).map_err(|error| error.to_string())?;
        if observation.schema != KEY || observation.owner.is_empty() {
            return Err("Unknown compiler pattern schema or missing owner".into());
        }

        check_edges(&observation.graph)?;
        Ok(observation)
    }
}

/// Shared field-preserving codec for advisory graphs, including saved review values.
pub fn read_graph(raw: &serde_json::Value) -> Result<Pdg, String> {
    require_graph(raw)?;
    let graph = Pdg::deserialize(raw).map_err(|error| error.to_string())?;
    check_edges(&graph)?;
    Ok(graph)
}

fn require_graph(raw: &serde_json::Value) -> Result<(), String> {
    let nodes = raw["nodes"]
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
    for edge in raw["edges"]
        .as_array()
        .ok_or("Missing compiler pattern edges")?
    {
        super::require_fields(edge, &["arg_pos"])?;
    }

    Ok(())
}

fn check_edges(graph: &Pdg) -> Result<(), String> {
    if graph
        .edges
        .iter()
        .any(|edge| edge.from >= graph.nodes.len() || edge.to >= graph.nodes.len())
    {
        return Err("Compiler pattern edge has no node".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
