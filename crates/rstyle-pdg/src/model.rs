//! Typed PDG/CFG interchange models for `compat.ccgraph.strict/0.1.0`.
//!
//! The structs mirror PDG 0.1.0 and CFG 0.2.0 structurally. Every fact keeps its
//! explicit status: known values stay values, and unknown/unsupported evidence
//! keeps its reason. Nothing is coerced to zero or an empty string here.
//! Semantic checks live in [`validate`](crate::validate) and [`cfg`].

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PDG_SCHEMA_VERSION: &str = "0.1.0";
pub const CFG_SCHEMA_VERSION: &str = "0.2.0";

/// First specified extraction policy; known but not implemented by any producer.
pub const R0_POLICY_ID: &str = "compat.rust.hir-r0";
pub const R0_POLICY_VERSION: &str = "0.1.0";

/// Reference-count policy fixed by the matching profile.
pub const REFERENCE_COUNT_POLICY_ID: &str = "compat.ccgraph.strict";
pub const REFERENCE_COUNT_POLICY_VERSION: &str = "0.1.0";

/// Required capability keys from the PDG contract.
pub const REQUIRED_CAPABILITIES: [&str; 15] = [
    "node_extraction",
    "scalar_reaching_definitions",
    "branch_merges",
    "loop_carried_dependencies",
    "control_dependencies",
    "execution_dependencies",
    "returns",
    "calls",
    "parameters",
    "closures",
    "macros",
    "memory_aliases",
    "exceptional_flow",
    "concurrency",
    "reference_counting",
];

/// Normal-flow capabilities that must be supported (or verified not applicable).
pub const REQUIRED_NORMAL_FLOW: [&str; 6] = [
    "node_extraction",
    "scalar_reaching_definitions",
    "control_dependencies",
    "execution_dependencies",
    "returns",
    "parameters",
];

pub type Extensions = BTreeMap<String, serde_json::Value>;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FactStatus {
    Known,
    Unknown,
    Unsupported,
    NotApplicable,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Supported,
    Approximated,
    Unsupported,
    Unknown,
    NotApplicable,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeCategory {
    Declaration,
    Read,
    Assignment,
    Computation,
    Control,
    Call,
    Entry,
    FormalInput,
    Return,
    FormalOutput,
    Exit,
    Merge,
    MemorySummary,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeFamily {
    Data,
    Control,
    Execution,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceOrigin {
    Source,
    Desugaring,
    MacroExpansion,
    Analysis,
    Projection,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamRole {
    Parameter,
    Receiver,
    Capture,
    Return,
    Output,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolRole {
    Parameter,
    Local,
    Global,
    Capture,
    Receiver,
    Output,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProfileId {
    pub id: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct StringFact {
    pub status: FactStatus,
    pub value: Option<String>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct BoolFact {
    pub status: FactStatus,
    pub value: Option<bool>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct CountFact {
    pub status: FactStatus,
    pub value: Option<u64>,
    pub policy: Option<ProfileId>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Capability {
    pub status: CapabilityStatus,
    pub description: String,
    pub policy: Option<ProfileId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Span {
    pub source_id: String,
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Provenance {
    pub origin: ProvenanceOrigin,
    pub description: String,
    #[serde(default)]
    pub origin_node_ids: Vec<String>,
    #[serde(default)]
    pub origin_spans: Vec<Span>,
    #[serde(default)]
    pub policy: Option<ProfileId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Access {
    #[serde(default)]
    pub symbol_id: Option<String>,
    #[serde(default)]
    pub location_id: Option<String>,
    #[serde(default)]
    pub unresolved_reason: Option<String>,
    #[serde(default)]
    pub operand_position: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct NodeAttributes {
    #[serde(default)]
    pub symbol_ids: Vec<String>,
    #[serde(default)]
    pub defines: Vec<String>,
    #[serde(default)]
    pub reads: Vec<Access>,
    #[serde(default)]
    pub writes: Vec<Access>,
    #[serde(default)]
    pub operator: Option<StringFact>,
    #[serde(default)]
    pub literal: Option<StringFact>,
    #[serde(default)]
    pub construct: Option<String>,
    #[serde(default)]
    pub write_form: Option<String>,
    #[serde(default)]
    pub position: Option<u64>,
    #[serde(default)]
    pub exit_kind: Option<String>,
    #[serde(default)]
    pub callee: Option<StringFact>,
    #[serde(default)]
    pub call_form: Option<String>,
    #[serde(default)]
    pub outcomes: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PdgNode {
    pub id: String,
    pub category: NodeCategory,
    pub original_kind: StringFact,
    pub synthetic: bool,
    #[serde(default)]
    pub source_span: Option<Span>,
    pub attributes: NodeAttributes,
    pub provenance: Provenance,
    pub evidence: Capability,
    #[serde(default)]
    pub annotations: Extensions,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PdgEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: EdgeFamily,
    pub subkind: String,
    pub policy: ProfileId,
    pub evidence: Capability,
    pub provenance: Provenance,
    #[serde(default)]
    pub operand_position: Option<u64>,
    #[serde(default)]
    pub symbol_id: Option<String>,
    #[serde(default)]
    pub definition_id: Option<String>,
    #[serde(default)]
    pub location_id: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub loop_carried: Option<bool>,
    #[serde(default)]
    pub annotations: Extensions,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Parameter {
    pub position: u64,
    pub role: ParamRole,
    #[serde(default)]
    pub symbol_id: Option<String>,
    pub name: StringFact,
    #[serde(rename = "type")]
    pub type_fact: StringFact,
    pub variadic: BoolFact,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Symbol {
    pub id: String,
    pub name: StringFact,
    pub scope_id: String,
    pub role: SymbolRole,
    #[serde(rename = "type")]
    pub type_fact: StringFact,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Definition {
    pub id: String,
    pub node_id: String,
    #[serde(default)]
    pub symbol_id: Option<String>,
    #[serde(default)]
    pub location_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Source {
    pub id: String,
    pub path: String,
    pub content_identity: StringFact,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct FunctionInfo {
    pub id: String,
    pub name: StringFact,
    pub qualified_name: StringFact,
    pub source_span: Span,
    pub inputs: Vec<Parameter>,
    pub outputs: Vec<Parameter>,
    pub interface_completeness: Capability,
    pub reference_count: CountFact,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct MemoryLocation {
    pub id: String,
    pub abstraction: String,
    pub description: String,
    pub evidence: Capability,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    #[serde(default)]
    pub source_span: Option<Span>,
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub capability: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ConformanceRecord {
    pub profile: ProfileId,
    pub status: String,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Producer {
    pub name: String,
    pub version: String,
    pub language: String,
    pub language_version: StringFact,
    #[serde(default)]
    pub configuration: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ExtractionEvidence {
    pub capabilities: BTreeMap<String, Capability>,
    pub diagnostics: Vec<Diagnostic>,
    pub conformance: Vec<ConformanceRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PdgDocument {
    pub schema_version: String,
    pub profile: ProfileId,
    pub producer: Producer,
    pub sources: Vec<Source>,
    pub function: FunctionInfo,
    pub symbols: Vec<Symbol>,
    pub definitions: Vec<Definition>,
    pub memory_locations: Vec<MemoryLocation>,
    pub nodes: Vec<PdgNode>,
    pub edges: Vec<PdgEdge>,
    pub extraction_evidence: ExtractionEvidence,
    #[serde(default)]
    pub extensions: Extensions,
}

impl StringFact {
    pub fn known(value: &str) -> Self {
        Self::known_owned(value.to_string())
    }

    pub fn known_owned(value: String) -> Self {
        Self {
            status: FactStatus::Known,
            value: Some(value),
            reason: None,
        }
    }

    pub fn missing(status: FactStatus, reason: &str) -> Self {
        Self {
            status,
            value: None,
            reason: Some(reason.to_string()),
        }
    }

    pub fn known_value(&self) -> Option<&str> {
        match self.status {
            FactStatus::Known => self.value.as_deref(),
            _ => None,
        }
    }

    /// Shape check only; semantic requiredness is decided by validation.
    pub fn shape_ok(&self) -> bool {
        well_formed(
            self.status == FactStatus::Known,
            self.value.is_some(),
            self.reason.is_some(),
        )
    }
}

/// Known facts carry a value and no reason; the rest carry a reason and no value.
fn well_formed(known: bool, has_value: bool, has_reason: bool) -> bool {
    if known {
        has_value && !has_reason
    } else {
        !has_value && has_reason
    }
}

impl BoolFact {
    pub fn known(value: bool) -> Self {
        Self {
            status: FactStatus::Known,
            value: Some(value),
            reason: None,
        }
    }

    pub fn shape_ok(&self) -> bool {
        well_formed(
            self.status == FactStatus::Known,
            self.value.is_some(),
            self.reason.is_some(),
        )
    }
}

impl CountFact {
    pub fn shape_ok(&self) -> bool {
        well_formed(
            self.status == FactStatus::Known,
            self.value.is_some(),
            self.reason.is_some(),
        ) && (self.status != FactStatus::Known || self.policy.is_some())
    }
}
