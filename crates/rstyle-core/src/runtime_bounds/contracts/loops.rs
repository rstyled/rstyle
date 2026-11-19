//! Loop forms and operator roles: how a language's loops that run their guard after each pass are
//! written, and which of its own operators core can evaluate, so core lowers both without knowing
//! the language.
use serde::Deserialize;

/// Where a loop form's guard is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Guard {
    /// The last statement of the loop's block, run after each pass; a `continue` goes to it, and
    /// the loop runs again while it holds.
    Last,
}

/// One loop form, by the frontend's original kind of the loop node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopForm {
    pub guard: Guard,
    /// Why the loop runs its guard there.
    pub basis: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LoopEntry {
    basis: String,
    guard: Guard,
    /// Original kinds of the loop node, without the namespace.
    pub(super) kinds: Vec<String>,
}

impl LoopEntry {
    pub(super) fn form(&self) -> LoopForm {
        LoopForm {
            guard: self.guard,
            basis: self.basis.clone(),
        }
    }
}

/// What a language's own operator computes, for core to evaluate it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum OperatorRole {
    /// Evaluates its operands in order, and has the value of the last.
    Sequence,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OperatorEntry {
    #[serde(rename = "basis")]
    _basis: String,
    pub(super) role: OperatorRole,
    /// Operator names, without the namespace.
    pub(super) operators: Vec<String>,
}
