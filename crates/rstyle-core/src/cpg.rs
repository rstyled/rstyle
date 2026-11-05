//! `compat.cpg` interchange documents: any language frontend emits them, and core
//! derives analysis inputs from them.
//!
//! This module derives the facts of the style rules, and the parameter details,
//! signature shapes, mutated types and dependence graphs the pattern analyses read.

pub mod bounds;
mod dependence;
mod events;
mod facts;
pub mod layout;
mod metrics;
pub mod model;
mod shapes;
pub mod source;
pub mod tree;

pub use facts::{CAPABILITIES, LAYERS, all_facts, rule_facts};

use crate::facts::FnFacts;

/// The runtime-bound program of every function in a run, by function identity.
pub fn run_bounds(
    documents: Vec<serde_json::Value>,
) -> Result<Vec<(String, crate::advisory::runtime_bounds::Input)>, String> {
    let document = layout::bundle(documents)?;
    let unit = document.unit.as_ref().ok_or("The document has no unit")?;
    let sources = source::Sources::new(unit)?;
    let tables = tree::Tables::new(unit)?;
    let arithmetic = match unit.semantics.integer_overflow.as_str() {
        "checked" => crate::runtime_bounds::Arithmetic::Checked,
        "wrapping" => crate::runtime_bounds::Arithmetic::Wrapping,
        _ => crate::runtime_bounds::Arithmetic::Unknown,
    };
    let pointer_bits = unit.target.pointer_width as u8;
    let functions = unit.functions.as_deref().unwrap_or_default();

    functions
        .iter()
        .map(|function| {
            let identity = tables
                .definition(&function.definition)?
                .stable_identity
                .clone();
            let lowered = tree::Body::new(function).and_then(|body| {
                let facts = bounds::facts::Facts {
                    body: &body,
                    tables: &tables,
                    sources: &sources,
                    pointer_bits,
                };
                bounds::lower(&facts, arithmetic)
            });
            // A function the lowering cannot read is unavailable, not a failed unit.
            let input = lowered.unwrap_or_else(|reason| {
                crate::advisory::runtime_bounds::Input::Unavailable {
                    reason: format!("compat.cpg lowering: {reason}"),
                }
            });
            Ok((identity, input))
        })
        .collect()
}

/// Rule facts of a run: one bundled document, or a split unit document and its
/// function documents.
pub fn run_facts(documents: Vec<serde_json::Value>) -> Result<Vec<FnFacts>, String> {
    rule_facts(&layout::bundle(documents)?)
}

#[cfg(test)]
mod dependence_tests;
#[cfg(test)]
mod shapes_tests;
#[cfg(test)]
mod tests;
