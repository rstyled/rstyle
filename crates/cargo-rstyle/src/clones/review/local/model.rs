//! Durable source-local review values contain no live proof or report handles.
use super::super::{CompilationTarget, Locator, Versions};
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Member {
    pub locator: Locator,
    pub context: BuildContext,
    /// Canonical original signature and caller structure, with local bindings normalized.
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Declaration {
    pub locator: Locator,
    /// Canonical original declaration tokens; method declarations exclude opaque bodies.
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Origin {
    Inherent,
    LocalTrait {
        declaration: Box<Declaration>,
        member: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Selection {
    pub owner: Declaration,
    pub method: Declaration,
    pub origin: Origin,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DispatchArm {
    pub variant: String,
    pub selection: Selection,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapabilityRole {
    pub left: Selection,
    pub right: Selection,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Proposal {
    EnumDispatch {
        enumeration: Declaration,
        arms: Vec<DispatchArm>,
    },
    OrderedCapability {
        roles: Vec<CapabilityRole>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Snapshot {
    pub versions: Versions,
    pub members: Vec<Member>,
    pub proposal: Proposal,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub finding: Snapshot,
    pub reason: String,
}
