//! Opt-in raw semantic CFG observations; no analysis result is capture authority.
use crate::runtime_bounds::Program;
use serde::{Deserialize, Serialize};

pub const KEY: &str = "rstyle.compiler-runtime-bounds/3";
pub const MAX_NODES: usize = 8192;
pub const MAX_VARIABLES: usize = 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub schema: String,
    pub owner: String,
    /// Compilation identity; owner paths alone collide across lib, bin and test builds.
    pub krate: String,
    pub input: Input,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Input {
    Captured { program: Program },
    BudgetExhausted { reason: String },
    Unavailable { reason: String },
}
