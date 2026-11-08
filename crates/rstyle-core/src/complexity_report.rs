//! Function scores and ranking, separate from optional threshold diagnostics.

use crate::{complexity::Measurement, facts::FnFacts, report::Diagnostic};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Function {
    pub id: String,
    pub name: String,
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub complexity: Option<Measurement>,
}

impl From<&FnFacts> for Function {
    fn from(fact: &FnFacts) -> Self {
        Self {
            id: fact.id.clone(),
            name: fact.name.clone(),
            path: fact.path.clone(),
            line: fact.line,
            column: fact.column,
            complexity: fact.complexity.clone(),
        }
    }
}

type Location<'a> = (&'a str, usize, usize, &'a str);
type RankKey<'a> = (std::cmp::Reverse<Option<(usize, usize)>>, Location<'a>);

fn rank_key<'a>(score: Option<&Measurement>, location: Location<'a>) -> RankKey<'a> {
    (
        std::cmp::Reverse(score.map(|s| (s.cognitive, s.cyclomatic))),
        location,
    )
}

/// Borrow all functions, including clean and unavailable owners, in complexity order.
pub fn ranked(facts: &[FnFacts]) -> Vec<&FnFacts> {
    let mut functions: Vec<_> = facts.iter().collect();
    rank_functions(&mut functions);
    functions
}

/// Keep functions with findings or unavailable analysis; clean measurements stay in facts.
pub fn findings(facts: &[FnFacts], diagnostics: &[Diagnostic]) -> Vec<Function> {
    let locations: BTreeSet<_> = diagnostics
        .iter()
        .map(|d| (d.path.as_str(), d.line, d.column))
        .collect();
    let mut functions: Vec<_> = facts
        .iter()
        .filter(|fact| {
            fact.complexity.is_none()
                || locations.contains(&(fact.path.as_str(), fact.line, fact.column))
        })
        .collect();
    rank_functions(&mut functions);
    functions.into_iter().map(Function::from).collect()
}

/// All functions are ranked by COG, then CC, then stable source identity.
fn rank_functions(functions: &mut [&FnFacts]) {
    functions
        .sort_by_key(|f| rank_key(f.complexity.as_ref(), (&f.path, f.line, f.column, &f.name)));
}

/// Compare completed rows with the same ranking used by the evaluator.
pub fn ordering(a: &Function, b: &Function) -> std::cmp::Ordering {
    rank_key(a.complexity.as_ref(), (&a.path, a.line, a.column, &a.name)).cmp(&rank_key(
        b.complexity.as_ref(),
        (&b.path, b.line, b.column, &b.name),
    ))
}

pub fn text(functions: &[Function]) -> Vec<String> {
    text_refs(functions)
}

/// Render completed rows borrowed from one or more evaluation reports.
pub fn text_refs<'a>(functions: impl IntoIterator<Item = &'a Function>) -> Vec<String> {
    let mut functions = functions.into_iter().peekable();
    if functions.peek().is_none() {
        return vec![];
    }
    let rows = functions.map(|function| {
        let scores = function.complexity.as_ref().map_or_else(
            || "   ?    ?".into(),
            |score| format!("{:4} {:4}", score.cyclomatic, score.cognitive),
        );
        format!(
            "{scores}  {}:{}:{}  {}",
            function.path, function.line, function.column, function.name
        )
    });
    std::iter::once("  CC  COG  LOCATION  FUNCTION".to_owned())
        .chain(rows)
        .collect()
}

#[cfg(test)]
mod tests;
