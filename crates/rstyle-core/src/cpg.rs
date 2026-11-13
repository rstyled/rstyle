//! `compat.cpg` interchange documents: any language frontend emits them, and core
//! derives analysis inputs from them.
//!
//! This module derives the facts of the style rules, and the parameter details,
//! signature shapes, mutated types and dependence graphs the pattern analyses read.

pub mod bounds;
pub mod defects;
mod dependence;
mod effects;
mod events;
mod facts;
pub mod failures;
pub mod flow;
pub mod layout;
mod metrics;
pub mod model;
mod pool;
pub mod records;
mod run;
mod shapes;
pub mod source;
mod stream;
pub mod transport;
pub mod tree;

pub use facts::{CAPABILITIES, LAYERS, all_facts, derivable_facts, rule_facts};
pub use pool::{OnRecords, Pool, keep_records, pooled_records};
pub use run::{RunInputs, run_inputs};
pub use stream::{read_records, visit_records};

use crate::facts::FnFacts;

/// The runtime-bound program of every function in a run, by function identity.
pub fn run_bounds(
    documents: Vec<serde_json::Value>,
) -> Result<Vec<(String, crate::advisory::runtime_bounds::Input)>, String> {
    lowered(&layout::bundle(documents)?)
}

/// What integer overflow means for a bound. An undefined overflow is one no defined execution
/// reaches, as a checked one is one no execution gets past, so a bound holds the same way.
fn arithmetic(overflow: &str) -> crate::runtime_bounds::Arithmetic {
    match overflow {
        "checked" | "undefined" => crate::runtime_bounds::Arithmetic::Checked,
        "wrapping" => crate::runtime_bounds::Arithmetic::Wrapping,
        "unbounded" => crate::runtime_bounds::Arithmetic::Unbounded,
        _ => crate::runtime_bounds::Arithmetic::Unknown,
    }
}

/// The runtime-bound program of every function of a document, in document order.
fn lowered(
    document: &model::Document,
) -> Result<Vec<(String, crate::advisory::runtime_bounds::Input)>, String> {
    let unit = document.unit.as_ref().ok_or("The document has no unit")?;
    let sources = source::Sources::new(unit)?;
    let tables = tree::Tables::new(unit)?;
    let arithmetic = arithmetic(&unit.semantics.integer_overflow);
    let pointer_bits = unit.target.pointer_width as u8;
    let destructors_are_calls = document
        .coverage
        .capabilities
        .get("implicit-calls")
        .is_some_and(|capability| capability.status == model::Status::Supported);
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
                    destructors_are_calls,
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

/// Rule facts of a run's authored functions, from any number of its units.
pub fn run_facts(documents: Vec<serde_json::Value>) -> Result<Vec<FnFacts>, String> {
    Ok(run_records(documents, records::Demand::default())?
        .into_iter()
        .filter_map(authored)
        .collect())
}

/// Every function of a run, from any number of its units, in document order.
pub fn run_records(
    documents: Vec<serde_json::Value>,
    demand: records::Demand,
) -> Result<Vec<records::Record>, String> {
    read_records(documents.into_iter().map(Ok), demand)
}

/// A record's measured facts, with its graph, when its function is authored.
pub fn authored(record: records::Record) -> Option<FnFacts> {
    match record.measured {
        records::Measured::Known(mut facts) => {
            facts.pdg = record.graph;
            Some(*facts)
        }
        _ => None,
    }
}

#[cfg(test)]
mod body_tests;
#[cfg(test)]
mod dependence_tests;
#[cfg(test)]
mod failures_tests;
#[cfg(test)]
mod flow_tests;
#[cfg(test)]
mod guard_clause_tests;
#[cfg(test)]
mod library_callee_tests;
#[cfg(test)]
mod py_effect_tests;
#[cfg(test)]
mod shapes_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod type_call_tests;
