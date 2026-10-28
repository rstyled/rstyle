//! Function scores and ranking, separate from optional threshold diagnostics.

use crate::{complexity::Measurement, facts::FnFacts};
use serde::Serialize;

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

#[derive(Clone, Copy)]
pub enum Order {
    Cyclomatic,
    Cognitive,
}

#[derive(Serialize)]
pub struct Ranking {
    pub total: usize,
    pub measured: usize,
    pub unavailable: usize,
    pub functions: Vec<Function>,
}

fn key(function: &Function, order: Order) -> (bool, usize, usize) {
    let Some(score) = &function.complexity else {
        return (false, 0, 0);
    };
    let (primary, secondary) = match order {
        Order::Cyclomatic => (score.cyclomatic, score.cognitive),
        Order::Cognitive => (score.cognitive, score.cyclomatic),
    };
    (true, primary, secondary)
}

impl Ranking {
    pub fn new(mut functions: Vec<Function>, order: Order, top: usize) -> Self {
        let total = functions.len();
        let measured = functions.iter().filter(|f| f.complexity.is_some()).count();
        functions.sort_by(|a, b| {
            key(b, order).cmp(&key(a, order)).then_with(|| {
                (&a.path, a.line, a.column, &a.name).cmp(&(&b.path, b.line, b.column, &b.name))
            })
        });
        functions.truncate(top);

        Self {
            total,
            measured,
            unavailable: total - measured,
            functions,
        }
    }

    pub fn text(&self) -> String {
        let rows = self.functions.iter().map(|function| {
            let scores = function.complexity.as_ref().map_or_else(
                || "   ?    ?".into(),
                |score| format!("{:4} {:4}", score.cyclomatic, score.cognitive),
            );
            format!(
                "{scores}  {}:{}:{}  {}",
                function.path, function.line, function.column, function.name
            )
        });
        let summary = format!(
            "complexity: {} functions, {} measured, {} unavailable",
            self.total, self.measured, self.unavailable
        );
        std::iter::once("  CC  COG  LOCATION  FUNCTION".to_owned())
            .chain(rows)
            .chain(std::iter::once(summary))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
