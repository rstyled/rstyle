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

fn complexity_rank(function: &FnFacts) -> Option<(usize, usize)> {
    function
        .complexity
        .as_ref()
        .map(|score| (score.cognitive, score.cyclomatic))
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
    functions.sort_by(|a, b| {
        complexity_rank(b).cmp(&complexity_rank(a)).then_with(|| {
            (&a.path, a.line, a.column, &a.name).cmp(&(&b.path, b.line, b.column, &b.name))
        })
    });
}

pub fn text(functions: &[Function]) -> Vec<String> {
    if functions.is_empty() {
        return vec![];
    }
    let rows = functions.iter().map(|function| {
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
