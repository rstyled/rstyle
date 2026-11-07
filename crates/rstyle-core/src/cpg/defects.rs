//! What a function's own facts show is probably wrong, for any frontend that emits them. Unlike a
//! style rule, a defect isn't a limit passed: it's a statement about what this code does.
//! A function whose facts fall short of a rule's needs stays unknown, and is never read as clean.

mod dead_store;

use super::flow::{self, Flow};
use super::model::{Coverage, Function};
use super::source::Sources;
use super::tree::Tables;
use serde::Serialize;

/// One thing a rule found in a function.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Defect {
    pub rule: &'static str,
    pub line: usize,
    pub message: String,
}

/// What the rules could say of a function.
#[derive(Clone, Debug)]
pub enum Examined {
    Read(Vec<Defect>),
    /// The function's facts can't answer, and why.
    Unknown(String),
}

/// What a rule reads of one function.
struct Reading<'a> {
    function: &'a Function,
    flow: Flow<'a>,
    tables: &'a Tables<'a>,
    sources: &'a Sources<'a>,
}

/// Runs every rule over a function. A malformed function is the document's fault, and reported
/// as unknown, with why, like any other function that can't be read.
pub fn examine(
    coverage: &Coverage,
    function: &Function,
    tables: &Tables<'_>,
    sources: &Sources<'_>,
) -> Examined {
    let flow = match flow::with_accesses(coverage, function) {
        Ok(flow) => flow,
        Err(reason) => return Examined::Unknown(reason),
    };
    let reading = Reading {
        function,
        flow,
        tables,
        sources,
    };
    match dead_store::find(&reading) {
        Ok(defects) => Examined::Read(defects),
        Err(reason) => Examined::Unknown(reason),
    }
}

#[cfg(test)]
mod tests;
