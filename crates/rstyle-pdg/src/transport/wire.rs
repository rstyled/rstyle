use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
#[serde(remote = "crate::CountFact", deny_unknown_fields)]
pub(super) struct CountFact {
    status: FactStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip)]
    policy: Option<ProfileId>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "crate::Capability", deny_unknown_fields)]
pub(super) struct Capability {
    #[serde(skip)]
    policy: Option<ProfileId>,
    status: CapabilityStatus,
    description: String,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "crate::Provenance", deny_unknown_fields)]
pub(super) struct Provenance {
    #[serde(skip)]
    policy: Option<ProfileId>,
    origin: ProvenanceOrigin,
    description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    origin_node_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    origin_spans: Vec<Span>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "crate::PdgNode", deny_unknown_fields)]
pub(super) struct PdgNode {
    id: String,
    category: NodeCategory,
    original_kind: StringFact,
    synthetic: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_span: Option<Span>,
    attributes: NodeAttributes,
    #[serde(with = "Provenance")]
    provenance: crate::Provenance,
    #[serde(with = "Capability")]
    evidence: crate::Capability,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    annotations: Extensions,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "crate::PdgEdge", deny_unknown_fields)]
pub(super) struct PdgEdge {
    #[serde(skip, default = "ProfileId::hir_r0")]
    policy: ProfileId,
    id: String,
    source: String,
    target: String,
    kind: EdgeFamily,
    subkind: String,
    #[serde(with = "Capability")]
    evidence: crate::Capability,
    #[serde(with = "Provenance")]
    provenance: crate::Provenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    operand_position: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    symbol_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    definition_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    location_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    outcome: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    loop_carried: Option<bool>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    annotations: Extensions,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "crate::FunctionInfo", deny_unknown_fields)]
pub(super) struct FunctionInfo {
    id: String,
    name: StringFact,
    qualified_name: StringFact,
    source_span: Span,
    inputs: Vec<Parameter>,
    outputs: Vec<Parameter>,
    #[serde(with = "Capability")]
    interface_completeness: crate::Capability,
    #[serde(with = "CountFact")]
    reference_count: crate::CountFact,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "crate::MemoryLocation", deny_unknown_fields)]
pub(super) struct MemoryLocation {
    id: String,
    abstraction: String,
    description: String,
    #[serde(with = "Capability")]
    evidence: crate::Capability,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    extensions: Extensions,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "crate::Conformance", deny_unknown_fields)]
pub(super) struct Conformance {
    #[serde(skip, default = "ProfileId::hir_r0")]
    profile: ProfileId,
    status: ConformanceStatus,
    reasons: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "crate::ExtractionEvidence", deny_unknown_fields)]
pub(super) struct ExtractionEvidence {
    #[serde(with = "super::records::capabilities")]
    capabilities: BTreeMap<String, crate::Capability>,
    diagnostics: Vec<Diagnostic>,
    #[serde(with = "super::conformance")]
    conformance: Vec<crate::Conformance>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "crate::PdgDocument", deny_unknown_fields)]
pub(super) struct Document {
    schema_version: String,
    profile: ProfileId,
    producer: Producer,
    sources: Vec<Source>,
    #[serde(with = "FunctionInfo")]
    function: crate::FunctionInfo,
    symbols: Vec<Symbol>,
    definitions: Vec<Definition>,
    #[serde(with = "super::records")]
    memory_locations: Vec<crate::MemoryLocation>,
    #[serde(with = "super::records")]
    nodes: Vec<crate::PdgNode>,
    #[serde(with = "super::records")]
    edges: Vec<crate::PdgEdge>,
    #[serde(with = "ExtractionEvidence")]
    extraction_evidence: crate::ExtractionEvidence,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    extensions: Extensions,
}
