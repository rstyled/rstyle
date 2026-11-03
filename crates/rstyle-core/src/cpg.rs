//! `compat.cpg` interchange documents: any language frontend emits them, and core
//! derives analysis inputs from them.
//!
//! This module derives the facts of the style rules, and the parameter details,
//! signature shapes and mutated types the pattern analyses read. Dependence graphs
//! are not derived yet, so they are left empty.

pub mod compare;
mod events;
mod facts;
pub mod layout;
mod metrics;
pub mod model;
mod shapes;
mod source;
mod tree;

pub use facts::{CAPABILITIES, LAYERS, rule_facts};

use crate::facts::FnFacts;

/// Rule facts of a run: one bundled document, or a split unit document and its
/// function documents.
pub fn run_facts(documents: Vec<serde_json::Value>) -> Result<Vec<FnFacts>, String> {
    rule_facts(&layout::bundle(documents)?)
}

#[cfg(test)]
mod shapes_tests;
#[cfg(test)]
mod tests;
