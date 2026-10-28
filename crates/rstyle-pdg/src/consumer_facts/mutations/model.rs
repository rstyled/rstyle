use super::super::Availability;
use crate::{ByteSpan, HirRef};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Assessment {
    pub availability: Availability<()>,
    pub coverage: Coverage,
    pub execution: Execution,
    pub sites: Vec<WriteSite>,
    pub boundaries: Vec<Boundary>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    Complete,
    Incomplete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Execution {
    Completed,
    BudgetExhausted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Root {
    pub declaration_span: ByteSpan,
    pub scope_span: ByteSpan,
    pub native: HirRef,
    pub formal: Option<u32>,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    LocalOwned,
    FormalOwned,
    FormalBorrowed,
}

#[derive(Debug, Serialize)]
pub struct WriteSite {
    pub span: ByteSpan,
    pub root: Root,
    pub operation: Operation,
    pub caller_visible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Assignment,
    CompoundAssignment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryAvailability {
    Unknown,
    Unsupported,
}

#[derive(Debug, Serialize)]
pub struct Boundary {
    pub availability: BoundaryAvailability,
    pub span: ByteSpan,
    pub reason: String,
}

impl Assessment {
    pub(super) fn empty() -> Self {
        Self {
            availability: Availability::Known(()),
            coverage: Coverage::Complete,
            execution: Execution::Completed,
            sites: vec![],
            boundaries: vec![],
        }
    }

    pub(super) fn boundary(&mut self, span: ByteSpan, reason: &str) {
        self.record_boundary(span, reason, BoundaryAvailability::Unsupported);
    }

    fn record_boundary(
        &mut self,
        span: ByteSpan,
        reason: &str,
        availability: BoundaryAvailability,
    ) {
        self.coverage = Coverage::Incomplete;
        self.boundaries.push(Boundary {
            span,
            availability,
            reason: reason.into(),
        });
    }
}

/// Optional call observations; direct mutation evidence keeps its original contract.
#[derive(Debug)]
pub struct Observations {
    pub direct: Assessment,
    pub calls: Vec<Call>,
    pub signature: Availability<()>,
}

#[derive(Debug)]
pub struct Call {
    pub span: ByteSpan,
    pub native: HirRef,
    /// Exact index of this call's retained boundary in `Observations::direct`.
    pub boundary_index: usize,
    pub target: Availability<String>,
    /// Checked native path, retained only for selected callee contradiction checks.
    pub target_path: Option<String>,
    pub arguments: Vec<Argument>,
}

#[derive(Debug)]
pub struct Argument {
    pub span: ByteSpan,
    pub value: Availability<ArgumentValue>,
}

#[derive(Debug)]
pub enum ArgumentValue {
    OwnedU32,
    MutableFormal(Root),
}
