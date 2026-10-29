//! Language-independent, source-level control-flow complexity (policy version 1).
//!
//! Frontends map syntax to this tree. They do not assign scores or nesting costs.
//! Sequence nodes preserve boundaries without adding complexity.

use serde::{Deserialize, Serialize};

/// Semantic constructs shared by language frontends, independent of their spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Sequence,
    Branch,
    ElseIf,
    Else,
    Loop,
    Selection,
    Alternative,
    Guard,
    Propagate,
    Jump,
    Recursion,
    Closure,
    Boolean,
    And,
    Or,
}

/// One construct and its source-ordered children. Locations are one-based.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub kind: Kind,
    pub line: usize,
    pub column: usize,
    pub children: Vec<Node>,
}

/// Evidence for one score increment, including the nesting cost used by COG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contribution {
    pub kind: Kind,
    pub line: usize,
    pub column: usize,
    pub cyclomatic: usize,
    pub cognitive: usize,
    pub nesting: usize,
}

/// A measured function. `None` at the facts boundary means unavailable, not zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Measurement {
    pub policy_version: u32,
    pub cyclomatic: usize,
    pub cognitive: usize,
    pub contributions: Vec<Contribution>,
}

struct Increment {
    cyclomatic: usize,
    cognitive: usize,
}

impl Increment {
    fn new(cyclomatic: usize, cognitive: usize) -> Self {
        Self {
            cyclomatic,
            cognitive,
        }
    }
}

impl Measurement {
    /// CC has a base of one; every other point in either score has source evidence.
    pub fn score(nodes: &[Node]) -> Self {
        let mut contributions = children(nodes, 0);
        contributions.sort_by_key(|point| (point.line, point.column));

        Self {
            policy_version: 1,
            cyclomatic: 1 + contributions
                .iter()
                .map(|point| point.cyclomatic)
                .sum::<usize>(),
            cognitive: contributions.iter().map(|point| point.cognitive).sum(),
            contributions,
        }
    }
}

fn point(node: &Node, nesting: usize, increment: Increment) -> Option<Contribution> {
    let Increment {
        cyclomatic,
        cognitive,
    } = increment;
    (cyclomatic > 0 || cognitive > 0).then_some(Contribution {
        kind: node.kind,
        line: node.line,
        column: node.column,
        cyclomatic,
        cognitive,
        nesting,
    })
}

fn children(nodes: &[Node], nesting: usize) -> Vec<Contribution> {
    nodes.iter().flat_map(|node| visit(node, nesting)).collect()
}

fn selection(node: &Node, nesting: usize) -> Vec<Contribution> {
    let scrutinee = node
        .children
        .iter()
        .filter(|child| child.kind != Kind::Alternative)
        .flat_map(|child| visit(child, nesting));
    let arms = node
        .children
        .iter()
        .filter(|child| child.kind == Kind::Alternative)
        .enumerate()
        .flat_map(|(index, arm)| {
            point(arm, nesting, Increment::new(usize::from(index > 0), 0))
                .into_iter()
                .chain(children(&arm.children, nesting + 1))
        });
    point(node, nesting, Increment::new(0, 1 + nesting))
        .into_iter()
        .chain(scrutinee)
        .chain(arms)
        .collect()
}

fn boolean(node: &Node, nesting: usize) -> Vec<Contribution> {
    let mut previous = None;
    node.children
        .iter()
        .flat_map(|child| {
            if !matches!(child.kind, Kind::And | Kind::Or) {
                return visit(child, nesting);
            }
            let increment = Increment::new(1, usize::from(previous != Some(child.kind)));
            previous = Some(child.kind);
            point(child, nesting, increment).into_iter().collect()
        })
        .collect()
}

fn increment(kind: Kind, nesting: usize) -> Increment {
    match kind {
        Kind::Branch | Kind::Loop | Kind::Guard => Increment::new(1, 1 + nesting),
        Kind::ElseIf => Increment::new(1, 1),
        Kind::Else | Kind::Jump | Kind::Recursion => Increment::new(0, 1),
        Kind::Propagate => Increment::new(1, 0),
        _ => Increment::new(0, 0),
    }
}

fn visit(node: &Node, nesting: usize) -> Vec<Contribution> {
    if node.kind == Kind::Selection {
        return selection(node, nesting);
    }
    if node.kind == Kind::Boolean {
        return boolean(node, nesting);
    }
    let nested = matches!(
        node.kind,
        Kind::Branch | Kind::Loop | Kind::Guard | Kind::ElseIf | Kind::Else | Kind::Closure
    );

    let contribution = point(node, nesting, increment(node.kind, nesting));
    contribution
        .into_iter()
        .chain(children(&node.children, nesting + usize::from(nested)))
        .collect()
}

#[cfg(test)]
mod tests;
