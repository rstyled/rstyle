//! What a function's own facts show is probably wrong, for any frontend that emits them. Unlike a
//! style rule, a defect isn't a limit passed: it's a statement about what this code does.
//! A function whose facts fall short of a rule's needs stays unknown, and is never read as clean.

mod dead_store;
pub mod deviant_argument;
mod identical_branches;
pub mod ignored_result;
pub mod taint;
pub mod value_flow;

use self::deviant_argument::Arguments;
use self::ignored_result::ResultUse;
use self::value_flow::ValueFlows;
use super::flow::{self, Flow, Reaching, Vertex};
use super::model::{Coverage, Function, Node};
use super::source::Sources;
use super::tree::{Body, Tables};
use serde::Serialize;
use std::collections::HashMap;
use std::rc::Rc;

/// One thing a rule found in a function.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Defect {
    pub rule: &'static str,
    pub severity: crate::config::Level,
    pub line: usize,
    pub message: String,
}

/// What the rules could say of a function.
#[derive(Clone, Debug)]
pub enum Examined {
    Read {
        defects: Vec<Defect>,
        /// What each call did with its result, which only the whole run can judge.
        results: Vec<ResultUse>,
        /// Where its values go, which a project's models say the meaning of.
        flows: ValueFlows,
        /// What its calls pass, which only its sibling methods can judge.
        arguments: Box<Arguments>,
    },
    /// The function's facts can't answer, and why.
    Unknown(String),
}

/// What a rule reads of one flow of a function: its own, or that of a closure written in it,
/// which shares the function's tree.
struct Reading<'a> {
    function: &'a Function,
    flow: Flow<'a>,
    body: Rc<Body<'a>>,
    parents: Rc<HashMap<&'a str, &'a Node>>,
    /// Which writes reach each vertex.
    reaching: Reaching<'a>,
    /// Each write with each read it reaches: where it's written, where it's read, what.
    dependence: Vec<(Vertex, Vertex, &'a str)>,
    tables: &'a Tables<'a>,
    sources: &'a Sources<'a>,
}

/// Runs every rule over a function. A malformed function is the document's fault, and reported
/// as unknown, with why, like any other function that can't be read.
pub fn examine(
    (coverage, namespace): (&Coverage, &str),
    function: &Function,
    tables: &Tables<'_>,
    sources: &Sources<'_>,
) -> Examined {
    let read = flow::with_accesses(coverage, function).and_then(|flow| {
        let reaching = Reaching::of(function, &flow);
        let dependence = reaching.data_dependence();
        let reading = Reading {
            function,
            flow,
            reaching,
            dependence,
            body: Rc::new(Body::new(function)?),
            parents: Rc::new(parents(function)),
            tables,
            sources,
        };
        let closures: Vec<Reading<'_>> = closures(&reading).collect();
        let mut defects = dead_store::find(&reading)?;
        for closure in &closures {
            defects.extend(dead_store::find(closure)?);
        }
        defects.extend(identical_branches::find(&reading)?);
        Ok(Examined::Read {
            defects,
            results: ignored_result::uses(&reading)?,
            flows: value_flow::of((&reading, &closures), namespace)?,
            arguments: Box::new(deviant_argument::of(&reading)?),
        })
    });
    read.unwrap_or_else(Examined::Unknown)
}

/// A reading of each closure the function records a flow for. A closure's body runs when it's
/// called, so what's stored and read there is judged in its own flow, where what it captures is
/// written at the entry.
fn closures<'r, 'a: 'r>(reading: &'r Reading<'a>) -> impl Iterator<Item = Reading<'a>> + 'r {
    let function = reading.function;
    let flows = function.cfg.closures.iter().filter_map(Flow::of_closure);
    flows.map(move |flow| {
        let reaching = Reaching::of(function, &flow);
        Reading {
            function,
            dependence: reaching.data_dependence(),
            reaching,
            flow,
            body: Rc::clone(&reading.body),
            parents: Rc::clone(&reading.parents),
            tables: reading.tables,
            sources: reading.sources,
        }
    })
}

fn parents(function: &Function) -> HashMap<&str, &Node> {
    function
        .nodes
        .iter()
        .flat_map(|node| {
            node.children
                .iter()
                .map(move |child| (child.as_str(), node))
        })
        .collect()
}

#[cfg(test)]
mod tests;
