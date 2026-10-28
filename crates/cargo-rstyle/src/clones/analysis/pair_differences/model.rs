use rstyle_pdg::{HirNode, HirRef};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Coverage {
    Complete,
    Unsupported,
    Ambiguous,
    BudgetExhausted,
}

/// Coverage is of this closed native expression comparison, never semantics.
#[derive(Debug)]
pub(crate) struct Assessment<'a> {
    pub coverage: Coverage,
    pub correspondence: Vec<Correspondence<'a>>,
    pub holes: Vec<Hole<'a>>,
    pub reason: Option<&'static str>,
}

impl Assessment<'_> {
    pub(super) fn unavailable(coverage: Coverage, reason: &'static str) -> Self {
        Self {
            coverage,
            correspondence: vec![],
            holes: vec![],
            reason: Some(reason),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct Correspondence<'a> {
    pub left: &'a HirRef,
    pub right: &'a HirRef,
    pub role: Role,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Role {
    Root,
    BlockTail,
    UnaryOperand,
    BinaryLeft,
    BinaryRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HoleKind {
    Literal,
    FormalRole,
    Operation,
    Shape,
}

#[derive(Debug)]
pub(crate) struct Hole<'a> {
    pub left: &'a HirNode,
    pub right: &'a HirNode,
    pub kind: HoleKind,
    pub role: Role,
}
