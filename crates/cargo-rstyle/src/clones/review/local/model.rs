//! Durable source-local review values contain no live proof or report handles.
use super::super::CompilationTarget;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BuildContext {
    pub compiler_commit: String,
    pub edition: String,
    pub data_layout: String,
    pub compilation_target: CompilationTarget,
    pub target_triple: String,
    pub pointer_width: u16,
    pub features: Vec<String>,
    pub cfgs: Vec<String>,
    pub opt_level: String,
    pub debug_assertions: bool,
    pub overflow_checks: bool,
    pub panic_strategy: String,
    pub extraction_policy: String,
}
