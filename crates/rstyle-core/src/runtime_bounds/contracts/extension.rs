//! Extension roles: how runtime bounds lower a node a language registers as its own, by the
//! extension's registered name, so core lowers it without knowing the language.
//!
//! Every role evaluates the node's children once, in the order the profile writes them. A role
//! only says when the node also runs an operation the document names no callee for, such as a
//! Python format call reaching a type's `__format__`. That operation can run code the library or
//! the workspace defines, so runtime bounds read it as an unresolved call at the node. A control
//! role says instead how the node raises an exception or handles one.
use serde::Deserialize;

/// When an extension node runs an operation of its own after its children.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum Implicit {
    /// Never: building the node's value runs only the language's own code.
    #[default]
    Never,
    /// Every time the node runs.
    Always,
    /// When the node names no member, as a `**` keyword argument unpacks a mapping.
    WithoutMember,
}

/// How an extension node raises an exception or handles one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Control {
    /// Evaluates its children, and then control does not pass: it raises what they made.
    Throw,
    /// Evaluates its first child, a test, and passes when it holds; otherwise evaluates the rest
    /// and raises.
    Assert,
    /// Runs its first child, its body, and its handlers when the body raises: a `match` among its
    /// children chooses one, and so does each extension the role names a handler. Every other
    /// child runs on every way out of the node, as a `finally` block does.
    Try,
    /// Enters each context its children name before its last child, its body, by evaluating it and
    /// then calling its enter method, with the pattern that follows it, if any, bound to what that
    /// returns. Every way out of the body calls each context's exit method, innermost first, and
    /// an exit method may end an exception the body raised.
    With,
}

/// One extension's role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extension {
    pub implicit: Implicit,
    pub control: Option<Control>,
    /// The namespaced extensions a `Try` reads as its handlers.
    pub handlers: Vec<String>,
    /// Why the node runs nothing else.
    pub basis: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExtensionEntry {
    basis: String,
    #[serde(default)]
    implicit: Implicit,
    control: Option<Control>,
    /// Extensions, without the namespace, that are a `Try`'s handlers.
    #[serde(default)]
    handlers: Vec<String>,
    /// Registered extension names, without the namespace.
    pub(super) extensions: Vec<String>,
}

impl ExtensionEntry {
    /// The role, once a control role runs no operation of its own and only a `Try` has handlers.
    pub(super) fn extension(&self, namespace: &str) -> Result<Extension, String> {
        let operates = self.control.is_some() && self.implicit != Implicit::Never;
        let handled = self.control != Some(Control::Try) && !self.handlers.is_empty();
        if operates || handled {
            return Err(format!(
                "Extension role {:?} with control {:?} has handlers {:?}",
                self.implicit, self.control, self.handlers
            ));
        }
        Ok(Extension {
            implicit: self.implicit,
            control: self.control,
            handlers: self
                .handlers
                .iter()
                .map(|handler| format!("{namespace}:{handler}"))
                .collect(),
            basis: self.basis.clone(),
        })
    }
}
