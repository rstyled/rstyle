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

impl Coverage {
    pub(in crate::consumer_facts) fn record_incomplete<T>(
        &mut self,
        outcomes: &mut Vec<T>,
        outcome: T,
    ) {
        *self = Self::Incomplete;
        outcomes.push(outcome);
    }
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
}

/// Optional call observations; direct mutation evidence keeps its original contract.
#[derive(Debug)]
pub struct Observations {
    pub direct: Assessment,
    pub calls: Vec<Call>,
    pub signature: Availability<()>,
    pub uses: UseInventory,
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
    /// Checked native free-function identity; independent of local summary permission.
    pub resolved_callee: Availability<ResolvedCallee>,
    pub arguments: Vec<Argument>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResolvedCallee {
    pub path: String,
    pub local: bool,
    pub safe: bool,
    pub result_type: String,
}

#[derive(Debug)]
pub struct Argument {
    pub span: ByteSpan,
    pub value: Availability<ArgumentValue>,
    /// Original caller formal ordinal for checked direct reads; locals/literals have none.
    pub formal: Availability<Option<u32>>,
    /// Checked immutable owned u32 formal read; no mutable, borrowed or alias roots.
    pub unchanged_formal: Availability<Option<u32>>,
}

#[derive(Debug)]
pub enum ArgumentValue {
    OwnedU32,
    MutableFormal(Root),
}

/// Original owned-u32 declarations, including checked zero-use formals.
#[derive(Debug, Serialize)]
pub struct UseFormal {
    pub ordinal: u32,
    pub span: ByteSpan,
    pub native: HirRef,
    pub mutable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UseRole {
    Read,
    Write,
    ReadWrite,
    SharedBorrow,
    MutableBorrow,
    CallArgument {
        call_index: usize,
        argument_index: usize,
        unchanged: bool,
    },
}

#[derive(Debug, Serialize)]
pub struct UseOccurrence {
    pub native: HirRef,
    pub span: ByteSpan,
    pub formal: u32,
    pub role: UseRole,
}

#[derive(Debug, Serialize)]
pub struct UseInventory {
    pub availability: Availability<()>,
    pub coverage: Coverage,
    pub execution: Execution,
    pub formals: Vec<UseFormal>,
    pub occurrences: Vec<UseOccurrence>,
    pub boundaries: Vec<Boundary>,
}

impl UseInventory {
    pub fn unrequested() -> Self {
        Self {
            availability: Availability::Unknown {
                reason: "Formal use inventory was not requested".into(),
            },
            coverage: Coverage::Incomplete,
            execution: Execution::Completed,
            formals: Vec::new(),
            occurrences: Vec::new(),
            boundaries: Vec::new(),
        }
    }
}
