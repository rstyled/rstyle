//! Persisted review values contain identity and user choices, never live proof.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Package {
    /// Workspace-relative Cargo.toml path, independent of the checkout root.
    pub identity: String,
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Locator {
    pub package: Package,
    pub source: String,
    pub qualified_name: String,
    pub declaration_kind: String,
    pub discriminator: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Context {
    pub compiler_commit: String,
    pub edition: String,
    pub data_layout: String,
    pub compilation_target: CompilationTarget,
    pub target_triple: String,
    pub pointer_width: u16,
    pub features: Vec<String>,
    pub cfgs: Vec<String>,
    pub opt_level: String,
    pub debug_assertions: bool,
    pub overflow_checks: bool,
    pub panic_strategy: String,
    pub extraction_policy: String,
    pub matching_policy: MatchingPolicy,
    pub consumer_policy: ConsumerPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompilationTarget {
    pub name: String,
    pub kind: Vec<String>,
    pub test: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MatchingPolicy {
    pub profile: String,
    pub minimum_size_ratio: String,
    pub numerical_threshold: String,
    pub string_threshold: String,
    pub final_threshold: String,
    pub scoring: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConsumerPolicy {
    pub id: String,
    pub version: u32,
    pub min_builtin_operations: usize,
    pub max_literal_holes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MemberKey {
    pub locator: Locator,
    pub context: Context,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ScalarType {
    U32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UnaryOperator {
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Expression {
    Formal {
        position: u32,
        ty: ScalarType,
    },
    Literal {
        ty: ScalarType,
        /// Canonical decimal semantic u32 value; radix and suffix live below.
        value: String,
        /// Deliberately relevant to identity: changing literal spelling stales
        /// review even when the semantic value remains equal.
        spelling: String,
    },
    Unary {
        ty: ScalarType,
        operator: UnaryOperator,
        operand: Box<Expression>,
    },
    Binary {
        ty: ScalarType,
        operator: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Signature {
    pub parameters: Vec<ScalarType>,
    pub result: ScalarType,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MemberSnapshot {
    pub key: MemberKey,
    pub signature: Signature,
    pub expression: Expression,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Versions {
    pub detector: String,
    pub detector_version: u32,
    pub identity_version: u32,
    pub locator_version: u32,
    pub context_version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FindingSnapshot {
    pub versions: Versions,
    pub members: Vec<MemberSnapshot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MembershipPolicy {
    Exact,
    AllowGrowth,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewEntry {
    pub finding: FindingSnapshot,
    pub membership: MembershipPolicy,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewStore {
    pub schema_version: u32,
    pub entries: Vec<ReviewEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_entries: Vec<super::local::Entry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameter_entries: Vec<super::source_rules::Entry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub length_entries: Vec<super::length_rules::Entry>,
}
