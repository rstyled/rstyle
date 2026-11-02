use super::Bound;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Program {
    pub inputs: Vec<String>,
    pub variables: Vec<Variable>,
    pub nodes: Vec<Node>,
    pub entry: u32,
    pub loop_headers: Vec<u32>,
    pub arithmetic: Arithmetic,
    /// Owner parameters in declaration order, for mapping summaries at call sites.
    pub formals: Vec<Formal>,
    /// Provenance of each length input.
    pub extents: Vec<Extent>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Formal {
    pub name: String,
    pub measure: Measure,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Measure {
    /// An unsigned parameter whose entry value is the named input.
    Scalar {
        input: String,
    },
    /// An immutable shared reference; places under it are stable while the owner runs.
    Shared,
    Opaque,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Extent {
    pub input: String,
    pub root: Root,
    pub fields: Vec<Field>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Root {
    Formal(u32),
    Local(u32),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub index: u32,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Callee {
    /// Compiler-resolved workspace definition, as `crate-identity/owner-path`.
    Resolved(String),
    /// A standard-library body that cannot call back into workspace code.
    Library(String),
    Constructor,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Argument {
    Scalar(Value),
    Place { root: Root, fields: Vec<Field> },
    Opaque,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Arithmetic {
    Checked,
    Wrapping,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub initial: Value,
    pub bits: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Value {
    Const(u64),
    Input(String),
    Variable(u32),
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub evidence: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub action: Action,
    pub flow: Flow,
    pub source: Option<Source>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Nop,
    Assign {
        variable: u32,
        value: Value,
    },
    Increment {
        variable: u32,
    },
    Call {
        callee: String,
        target: Callee,
        arguments: Vec<Argument>,
        may_modify: bool,
    },
    Unsupported {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condition {
    Less { variable: u32, limit: Value },
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Flow {
    Goto(u32),
    Branch {
        condition: Condition,
        then_node: u32,
        else_node: u32,
    },
    Return,
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub nodes: usize,
    pub variables: usize,
    pub iterations: usize,
    pub expression_nodes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            nodes: 2048,
            variables: 128,
            iterations: 4096,
            expression_nodes: 4096,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Proven(Assessment),
    Unknown { reason: String },
    BudgetExceeded { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assessment {
    pub cost_scope: String,
    pub loop_body_entries: Bound,
    pub loops: Vec<LoopBound>,
    pub calls: Vec<CallBound>,
    pub whole_runtime: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopBound {
    pub header: u32,
    pub source: Option<Source>,
    pub body_entries: Bound,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallBound {
    pub node: u32,
    pub callee: String,
    pub target: Callee,
    pub source: Option<Source>,
    pub count: Bound,
    /// Per-invocation upper bounds or places for each argument at this site.
    pub arguments: Vec<ArgumentBound>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArgumentBound {
    Scalar(Bound),
    Place { root: Root, fields: Vec<Field> },
    Unknown,
}
