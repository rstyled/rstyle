//! Typed PDG interchange models for PDG 0.1.0.
//!
//! The structs mirror PDG 0.1.0 structurally with closed objects
//! (`deny_unknown_fields`) and absent-optional omission
//! (`skip_serializing_if`) so serialized wire documents validate against the
//! authoritative schema. Every fact keeps its explicit status: known values
//! stay values, and unknown/unsupported evidence keeps a nonempty reason.
//! Nothing is coerced to zero or an empty string here. Semantic checks live
//! in [`validate`](crate::validate); CFG lives in [`cfg`](crate::cfg).

use serde::{Deserialize, Deserializer, Serialize};
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

/// Namespaced extension keys per the PDG contract:
/// `^[a-zA-Z][a-zA-Z0-9_.-]*:[a-zA-Z0-9_.-]+$`.
pub fn is_valid_extension_key(key: &str) -> bool {
    let Some((prefix, suffix)) = key.split_once(':') else {
        return false;
    };

    valid_prefix(prefix) && valid_suffix(suffix)
}

fn valid_prefix(prefix: &str) -> bool {
    let mut chars = prefix.chars();

    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }

    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

fn valid_suffix(suffix: &str) -> bool {
    !suffix.is_empty()
        && suffix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Extension maps with invalid namespaces are rejected at validation time;
/// transport preserves them verbatim so diagnostics can report the key.
pub fn invalid_extension_keys(extensions: &Extensions) -> Vec<String> {
    extensions
        .keys()
        .filter(|key| !is_valid_extension_key(key))
        .cloned()
        .collect()
}

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

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitKind {
    Normal,
    Exceptional,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CallForm {
    Direct,
    Indirect,
    Unknown,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConformanceStatus {
    Conformant,
    NonConformant,
    NotEvaluated,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileId {
    pub id: String,
    pub version: String,
}

// ---- Facts: known {status,value} vs missing {status,reason} ----

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StringFact {
    pub status: FactStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BoolFact {
    pub status: FactStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CountFact {
    pub status: FactStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<ProfileId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct StringFactWire {
    status: FactStatus,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoolFactWire {
    status: FactStatus,
    #[serde(default)]
    value: Option<bool>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct CountFactWire {
    status: FactStatus,
    #[serde(default)]
    value: Option<u64>,
    #[serde(default)]
    policy: Option<ProfileId>,
    #[serde(default)]
    reason: Option<String>,
}

fn reject_empty_reason(reason: &Option<String>) -> Result<(), String> {
    match reason {
        Some(text) if !text.is_empty() => Ok(()),
        Some(_) => Err("reason must be nonempty".to_string()),
        None => Err("missing reason for non-known fact".to_string()),
    }
}

impl<'de> Deserialize<'de> for StringFact {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = StringFactWire::deserialize(deserializer)?;

        match wire.status {
            FactStatus::Known => {
                if wire.value.is_none() || wire.reason.is_some() {
                    return Err(serde::de::Error::custom(
                        "known string fact requires value and no reason",
                    ));
                }
            }
            _ => {
                if wire.value.is_some() {
                    return Err(serde::de::Error::custom(
                        "non-known string fact must not carry a value",
                    ));
                }

                reject_empty_reason(&wire.reason).map_err(serde::de::Error::custom)?;
            }
        }

        Ok(Self {
            status: wire.status,
            value: wire.value,
            reason: wire.reason,
        })
    }
}

impl<'de> Deserialize<'de> for BoolFact {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = BoolFactWire::deserialize(deserializer)?;

        match wire.status {
            FactStatus::Known => {
                if wire.value.is_none() || wire.reason.is_some() {
                    return Err(serde::de::Error::custom(
                        "known boolean fact requires value and no reason",
                    ));
                }
            }
            _ => {
                if wire.value.is_some() {
                    return Err(serde::de::Error::custom(
                        "non-known boolean fact must not carry a value",
                    ));
                }

                reject_empty_reason(&wire.reason).map_err(serde::de::Error::custom)?;
            }
        }

        Ok(Self {
            status: wire.status,
            value: wire.value,
            reason: wire.reason,
        })
    }
}

impl<'de> Deserialize<'de> for CountFact {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = CountFactWire::deserialize(deserializer)?;

        match wire.status {
            FactStatus::Known => {
                if wire.value.is_none() || wire.policy.is_none() || wire.reason.is_some() {
                    return Err(serde::de::Error::custom(
                        "known count fact requires value, policy and no reason",
                    ));
                }
            }
            _ => {
                if wire.value.is_some() || wire.policy.is_some() {
                    return Err(serde::de::Error::custom(
                        "non-known count fact must not carry value or policy",
                    ));
                }

                reject_empty_reason(&wire.reason).map_err(serde::de::Error::custom)?;
            }
        }

        Ok(Self {
            status: wire.status,
            value: wire.value,
            policy: wire.policy,
            reason: wire.reason,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub status: CapabilityStatus,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<ProfileId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    pub source_id: String,
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub origin: ProvenanceOrigin,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub origin_node_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub origin_spans: Vec<Span>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<ProfileId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Access {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operand_position: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeAttributes {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub symbol_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub defines: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reads: Vec<Access>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub writes: Vec<Access>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<StringFact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub literal: Option<StringFact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub construct: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write_form: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_kind: Option<ExitKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callee: Option<StringFact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_form: Option<CallForm>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outcomes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: Extensions,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PdgNode {
    pub id: String,
    pub category: NodeCategory,
    pub original_kind: StringFact,
    pub synthetic: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_span: Option<Span>,
    pub attributes: NodeAttributes,
    pub provenance: Provenance,
    pub evidence: Capability,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: Extensions,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PdgEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: EdgeFamily,
    pub subkind: String,
    pub policy: ProfileId,
    pub evidence: Capability,
    pub provenance: Provenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operand_position: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_carried: Option<bool>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: Extensions,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub position: u64,
    pub role: ParamRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_id: Option<String>,
    pub name: StringFact,
    #[serde(rename = "type")]
    pub type_fact: StringFact,
    pub variadic: BoolFact,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: Extensions,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Symbol {
    pub id: String,
    pub name: StringFact,
    pub scope_id: String,
    pub role: SymbolRole,
    #[serde(rename = "type")]
    pub type_fact: StringFact,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: Extensions,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub id: String,
    pub node_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub path: String,
    pub content_identity: StringFact,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct MemoryLocation {
    pub id: String,
    pub abstraction: String,
    pub description: String,
    pub evidence: Capability,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: Extensions,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_span: Option<Span>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConformanceRecord {
    pub profile: ProfileId,
    pub status: ConformanceStatus,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Producer {
    pub name: String,
    pub version: String,
    pub language: String,
    pub language_version: StringFact,
    pub configuration: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionEvidence {
    pub capabilities: BTreeMap<String, Capability>,
    pub diagnostics: Vec<Diagnostic>,
    pub conformance: Vec<ConformanceRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
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
            self.reason.as_deref(),
        )
    }
}

/// Known facts carry a value and no reason; the rest carry a nonempty reason
/// and no value.
fn well_formed(known: bool, has_value: bool, reason: Option<&str>) -> bool {
    if known {
        has_value
    } else {
        !has_value && reason.is_some_and(|text| !text.is_empty())
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

    pub fn missing(status: FactStatus, reason: &str) -> Self {
        Self {
            status,
            value: None,
            reason: Some(reason.to_string()),
        }
    }

    pub fn shape_ok(&self) -> bool {
        well_formed(
            self.status == FactStatus::Known,
            self.value.is_some(),
            self.reason.as_deref(),
        )
    }
}

impl CountFact {
    pub fn known(value: u64, policy: ProfileId) -> Self {
        Self {
            status: FactStatus::Known,
            value: Some(value),
            policy: Some(policy),
            reason: None,
        }
    }

    pub fn missing(status: FactStatus, reason: &str) -> Self {
        Self {
            status,
            value: None,
            policy: None,
            reason: Some(reason.to_string()),
        }
    }

    pub fn shape_ok(&self) -> bool {
        well_formed(
            self.status == FactStatus::Known,
            self.value.is_some(),
            self.reason.as_deref(),
        ) && (self.status != FactStatus::Known || self.policy.is_some())
    }
}
