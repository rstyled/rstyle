//! Direct collector events for expanded owners without authored source metrics.
use crate::facts::{CallEvent, Edge, EffectEvent, MutationEvent};
use serde::{Deserialize, Serialize};

pub const KEY: &str = "rstyle.compiler-generated-effects/1";
pub const CONNECTED_KEY: &str = "rstyle.compiler-generated-effects/2";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connections {
    pub id: String,
    pub implements: Option<String>,
    pub edges: Vec<Edge>,
}

/// Raw collector events and optional connections; no authored size or purity claim.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub metrics: String,
    pub name: String,
    pub line: usize,
    pub column: usize,
    pub mutations: Vec<MutationEvent>,
    pub calls: Vec<CallEvent>,
    pub intrinsic_effects: Vec<EffectEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connections: Option<Connections>,
    /// The build wrote the file it is in, so path exclusions, which select original
    /// source, do not apply to it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub generated_file: bool,
}
