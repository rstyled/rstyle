//! Compiler-resolved module expansion links into the immutable source census.
use crate::ByteSpan;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSpan {
    pub path: PathBuf,
    pub digest: String,
    pub span: ByteSpan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpansionStep {
    pub macro_definition: String,
    pub definition: FileSpan,
    pub invocation: FileSpan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleExpansion {
    pub module: String,
    pub parent: String,
    pub declaration: FileSpan,
    pub loaded_file: PathBuf,
    pub chain: Vec<ExpansionStep>,
}
