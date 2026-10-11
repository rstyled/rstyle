//! Typed R0 fixture expectations with selector resolution.
//!
//! The structs mirror the vendored `rust-r0-extraction.json` format
//! `compat.rust.hir-r0.fixture-expectations / 0.1.0` with closed objects so
//! accidental edits to the vendored snapshot fail loudly instead of
//! silently changing the oracle. Resolution of every selector to byte spans
//! uses only [`crate::selector`], never producer routines.

use crate::selector;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Path of the vendored normative fixture file, relative to the crate root.
pub fn vendored_fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor-compat/rust-r0/fixtures/rust-r0-extraction.json")
}

/// Parse fixture bytes (exact snapshot content, no added newline).
pub fn parse_pack(bytes: &[u8]) -> FixturePack {
    serde_json::from_slice(bytes).expect("parse vendored R0 fixture file")
}

/// Load and parse the vendored fixture pack byte-for-byte as JSON.
pub fn load_vendored_pack() -> FixturePack {
    let path = vendored_fixture_path();
    let bytes = std::fs::read(&path).expect("read vendored R0 fixture file");
    parse_pack(&bytes)
}

/// `[exact token sequence, 1-based occurrence]` per the fixture encoding.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SourceSelector(pub String, pub usize);

impl SourceSelector {
    /// Resolve against exact source bytes to a half-open byte span.
    pub fn resolve(&self, source: &str) -> Option<(usize, usize)> {
        selector::resolve(source, &self.0, self.1)
    }
}

/// `[alias, PDG category, source selector or null for analysis merges]`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct NodeExpect(pub String, pub String, pub Option<SourceSelector>);

/// `[symbol alias, declared name, resolved type, role, declaring selector]`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SymbolExpect(
    pub String,
    pub String,
    pub String,
    pub String,
    pub SourceSelector,
);

/// `[definition ID, symbol alias, defining node alias]`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DefinitionExpect(pub String, pub String, pub String);

/// `[from alias, to alias, CFG kind]` (`E`/`X` are the boundaries).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CfgEdgeExpect(pub String, pub String, pub String);

/// `[defining node, read node, definition ID]`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ReachingExpect(pub String, pub String, pub String);

/// `[producer, consumer, operand position]`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ValueEdgeExpect(pub String, pub String, u32);

/// `[producer, merge, alternative position, outcome, controller]`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct AlternativeEdgeExpect(pub String, pub String, u32, pub String, pub String);

/// `[prerequisite, target]`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ExecutionEdgeExpect(pub String, pub String);

/// `[controller, controlled operation, outcome]`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ControlEdgeExpect(pub String, pub String, pub String);

/// One DNF atom: the controller executed and took this outcome.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct GuardAtom {
    pub controller_pdg_node_id: String,
    pub outcome: String,
}

/// DNF availability/selection guard: outer OR of inner ANDs.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct GuardAnnotations {
    #[serde(rename = "compat.rust.hir-r0:guards")]
    pub guards: Vec<Vec<GuardAtom>>,
}

/// One merge alternative with its completion frontier and selection guard.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct AlternativeExpect {
    pub value_pdg_node_id: String,
    pub outcome: String,
    pub completion_cfg_node_ids: Vec<String>,
    pub annotations: GuardAnnotations,
}

/// `compat.rust.hir-r0:value-sites` metadata for one expression merge.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ValueSiteExpect {
    pub pdg_node_id: String,
    pub construct_selector: SourceSelector,
    pub controller_pdg_node_id: String,
    pub cfg_successor_node_id: String,
    pub annotations: GuardAnnotations,
    pub alternatives: Vec<AlternativeExpect>,
}

/// One ineligibility diagnostic with its source evidence selector.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DiagnosticExpect {
    pub capability: String,
    pub selector: SourceSelector,
    pub severity: String,
    pub reason: String,
}

/// One output interface element (`position`, `role`, `type`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct OutputExpect {
    pub position: u32,
    pub role: String,
    #[serde(rename = "type")]
    pub type_name: String,
}

/// Exact per-symbol IN/OUT definition sets keyed by node alias.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DefinitionSets {
    #[serde(rename = "in")]
    pub inn: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    pub out: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

/// Unreachable diagnostic value edges: allowed producer/consumer/position
/// triples that must never be certified as reachable facts.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DiagnosticValueEdges {
    pub allowed: Vec<ValueEdgeExpect>,
    pub required: bool,
    pub certified_reachable_edges: bool,
}

/// All three PDG edge families plus execution and control prerequisites.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct PdgEdgesExpect {
    pub reaching_definition: Vec<ReachingExpect>,
    pub value_operand: Vec<ValueEdgeExpect>,
    pub value_alternative: Vec<AlternativeEdgeExpect>,
    pub execution: Vec<ExecutionEdgeExpect>,
    pub control: Vec<ControlEdgeExpect>,
}

/// Expected facts for one fixture case: accepted graphs or rejections.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub eligibility: String,
    pub conformance: Option<String>,
    pub exact_graph: Option<bool>,
    pub cfg_evidence_status: Option<String>,
    pub limitations: Option<Vec<String>>,
    pub exceptional_flow_evidence_required: Option<String>,
    pub symbols: Option<Vec<SymbolExpect>>,
    pub nodes: Option<Vec<NodeExpect>>,
    pub definitions: Option<Vec<DefinitionExpect>>,
    pub read_definitions: Option<BTreeMap<String, Vec<String>>>,
    pub local_definition_sets: Option<DefinitionSets>,
    pub cfg_edges: Option<Vec<CfgEdgeExpect>>,
    pub pdg_edges: Option<PdgEdgesExpect>,
    pub value_sites: Option<Vec<ValueSiteExpect>>,
    pub reference_count: Option<u32>,
    pub outputs: Option<Vec<OutputExpect>>,
    pub literal_attributes: Option<BTreeMap<String, String>>,
    pub operator_attributes: Option<BTreeMap<String, String>>,
    pub source_operation_roles: Option<BTreeMap<String, String>>,
    pub reachable_operations: Option<Vec<String>>,
    pub unreachable_operations: Option<Vec<String>>,
    pub projection_removed_operations: Option<Vec<String>>,
    pub unreachable_diagnostic_value_edges: Option<DiagnosticValueEdges>,
    pub diagnostics: Option<Vec<DiagnosticExpect>>,
    pub outer_function_operation_exclusions: Option<Vec<SourceSelector>>,
    #[serde(default)]
    pub forbidden: Vec<String>,
}

/// One fixture case: exact source plus independent expectations.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct FixtureCase {
    pub id: String,
    pub source: String,
    #[serde(default)]
    pub context: Option<String>,
    pub execution_status: String,
    pub expected: Expected,
}

impl FixtureCase {
    /// `sha256:<lowercase hex>` of the exact source bytes (no added newline).
    pub fn source_identity(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.source.as_bytes());
        format!("sha256:{:x}", hasher.finalize())
    }

    /// Resolve every node selector; merges (`null`) map to `None`.
    pub fn resolved_node_spans(&self) -> BTreeMap<String, Option<(usize, usize)>> {
        self.expected
            .nodes
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|node| {
                let span = node.2.as_ref().and_then(|sel| sel.resolve(&self.source));
                (node.0, span)
            })
            .collect()
    }

    /// Resolve every symbol declaring-identifier selector.
    pub fn resolved_symbol_spans(&self) -> BTreeMap<String, (usize, usize)> {
        self.expected
            .symbols
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|symbol| symbol.4.resolve(&self.source).map(|span| (symbol.0, span)))
            .collect()
    }

    /// Every selector the harness must resolve for this case.
    pub fn all_selectors(&self) -> Vec<SourceSelector> {
        let expected = &self.expected;
        let symbols = expected
            .symbols
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|symbol| symbol.4);
        let nodes = expected
            .nodes
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|node| node.2);
        let sites = expected
            .value_sites
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|site| site.construct_selector);
        let diagnostics = expected
            .diagnostics
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|diagnostic| diagnostic.selector);
        let exclusions = expected
            .outer_function_operation_exclusions
            .clone()
            .unwrap_or_default()
            .into_iter();

        symbols
            .chain(nodes)
            .chain(sites)
            .chain(diagnostics)
            .chain(exclusions)
            .collect()
    }
}

/// Versioned fixture-format marker.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct FormatMarker {
    pub id: String,
    pub version: String,
}

/// Profile under test: `compat.rust.hir-r0 / 0.1.0`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ProfileMarker {
    pub id: String,
    pub version: String,
}

/// Schema targets: CFG `0.2.0`, PDG `0.1.0`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SchemaTargets {
    pub cfg: String,
    pub pdg: String,
}

/// Availability evidence that is still unknown at authoring time.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct UnknownEvidence {
    pub status: String,
    pub reason: String,
}

/// Fixture-run configuration: a test input description, not toolchain proof.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct FixtureConfiguration {
    pub language: String,
    pub edition: String,
    pub overflow_checks: bool,
    pub debug_assertions: bool,
    pub panic_strategy: String,
    pub optimization: String,
    pub cfg: Vec<String>,
    pub cargo_features: Vec<String>,
    pub target_pointer_width: u32,
    pub rustc_toolchain: UnknownEvidence,
    pub target_triple: UnknownEvidence,
    pub preconditions: String,
}

/// The full vendored expectation pack: 23 independent hand-authored cases.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct FixturePack {
    pub fixture_format: FormatMarker,
    pub extraction_profile: ProfileMarker,
    pub schema_targets: SchemaTargets,
    pub configuration: FixtureConfiguration,
    pub encoding: serde_json::Value,
    pub authoring: String,
    pub cases: Vec<FixtureCase>,
    pub validation: serde_json::Value,
}
