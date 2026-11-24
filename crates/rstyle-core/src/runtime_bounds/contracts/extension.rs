//! Extension roles: how runtime bounds lower a node a language registers as its own, by the
//! extension's registered name, so core lowers it without knowing the language.
//!
//! Every role evaluates the node's children once, in the order the profile writes them. A role
//! only says when the node also runs an operation the document names no callee for, such as a
//! Python format call reaching a type's `__format__`. That operation can run code the library or
//! the workspace defines, so runtime bounds read it as an unresolved call at the node. A control
//! role says instead how the node runs its children: how it raises an exception or handles one,
//! runs a call later, removes what it names, or chains comparisons.
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

/// How an extension node runs its children, other than in order.
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
    /// Evaluates its child, a call, where it's written, and runs the call later: when the function
    /// exits, or on a thread of its own.
    Deferred,
    /// Removes each child in order: a name is unbound, and an item or an attribute is removed by a
    /// method its container defines.
    Delete,
    /// Compares each child with the next, evaluating each once and the next only while every
    /// comparison so far held. Comparing values of other than primitive types runs a method they
    /// define.
    Chain,
}

/// One extension's role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extension {
    pub implicit: Implicit,
    pub control: Option<Control>,
    /// The namespaced extensions a `Try` reads as its handlers.
    pub handlers: Vec<String>,
    /// The namespaced modifier of a node that runs where what it makes is consumed, rather than
    /// where it's written, as a generator expression does.
    pub lazy: Option<String>,
    /// The namespaced operations that drain a lazy node handed to them as their first positional
    /// argument before they return, and keep nothing of it, as `sum` does.
    pub consumed_by: Vec<String>,
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
    /// The modifier, without the namespace, of a node that runs where it's consumed.
    lazy: Option<String>,
    /// Operations, without the namespace, that drain a lazy node handed to them first.
    #[serde(default)]
    consumed_by: Vec<String>,
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
        if self.lazy.is_none() && !self.consumed_by.is_empty() {
            return Err(format!(
                "Extension role that isn't lazy is consumed by {:?}",
                self.consumed_by
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
            lazy: self
                .lazy
                .as_ref()
                .map(|modifier| format!("{namespace}:{modifier}")),
            consumed_by: self
                .consumed_by
                .iter()
                .map(|operation| format!("{namespace}:{operation}"))
                .collect(),
            basis: self.basis.clone(),
        })
    }
}
