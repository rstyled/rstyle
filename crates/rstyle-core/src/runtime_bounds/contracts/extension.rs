//! Extension roles: how runtime bounds lower a node a language registers as its own, by the
//! extension's registered name, so core lowers it without knowing the language.
//!
//! Every role evaluates the node's children once, in the order the profile writes them. A role
//! only says when the node also runs an operation the document names no callee for, such as a
//! Python format call reaching a type's `__format__`. That operation can run code the library or
//! the workspace defines, so runtime bounds read it as an unresolved call at the node.
use serde::Deserialize;

/// When an extension node runs an operation of its own after its children.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Implicit {
    /// Never: building the node's value runs only the language's own code.
    Never,
    /// Every time the node runs.
    Always,
    /// When the node names no member, as a `**` keyword argument unpacks a mapping.
    WithoutMember,
}

/// One extension's role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extension {
    pub implicit: Implicit,
    /// Why the node runs nothing else.
    pub basis: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExtensionEntry {
    basis: String,
    implicit: Implicit,
    /// Registered extension names, without the namespace.
    pub(super) extensions: Vec<String>,
}

impl ExtensionEntry {
    pub(super) fn extension(&self) -> Extension {
        Extension {
            implicit: self.implicit,
            basis: self.basis.clone(),
        }
    }
}
