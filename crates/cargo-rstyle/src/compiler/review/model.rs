//! Persisted review values contain identity and user choices, never live proof.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Package {
    /// Workspace-relative Cargo.toml path, independent of the checkout root.
    pub identity: String,
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Locator {
    pub package: Package,
    pub source: String,
    pub qualified_name: String,
    pub declaration_kind: String,
    pub discriminator: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompilationTarget {
    pub name: String,
    pub kind: Vec<String>,
    pub test: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Versions {
    pub detector: String,
    pub detector_version: u32,
    pub identity_version: u32,
    pub locator_version: u32,
    pub context_version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReviewStore {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compiler_pattern_entries: Vec<super::compiler_patterns::Entry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compiler_diagnostic_entries: Vec<super::compiler_diagnostics::Entry>,
}
