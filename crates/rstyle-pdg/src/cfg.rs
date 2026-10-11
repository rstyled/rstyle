//! CFG interchange model, reachability, and CFG/PDG correspondence.
//!
//! The model mirrors CFG 0.2.0 structurally with closed objects and
//! absent-optional omission. The matcher derives operation reachability from
//! the paired CFG's entry, never from array order. Correspondence triples
//! live in the shared `compat.rust.hir-r0:operation-map` extension on both
//! documents.

use crate::model::{Extensions, Span, StringFact};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const OPERATION_MAP_KEY: &str = "compat.rust.hir-r0:operation-map";
pub const PAIRED_GRAPH_KEY: &str = "compat.rust.hir-r0:paired-graph";

/// R0 CFG construction policy identity.
pub const R0_CFG_NAME: &str = "compat.rust.hir-r0";
pub const R0_CFG_VERSION: &str = "0.1.0";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CfgNodeKind {
    Entry,
    Exit,
    Block,
    Condition,
    Transfer,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CfgExitKind {
    Normal,
    Exceptional,
    ProcessTermination,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CfgEdgeKind {
    Normal,
    True,
    False,
    Case,
    Default,
    Return,
    Break,
    Continue,
    Exception,
    Terminate,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopRole {
    BackEdge,
    LoopExit,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Granularity {
    Operation,
    BasicBlock,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExceptionalFlow {
    Explicit,
    Conservative,
    Omitted,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnreachableCode {
    Preserve,
    Omit,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortCircuit {
    ExplicitBranches,
    Reject,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnspecifiedOrder {
    Overapproximate,
    Reject,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionStatus {
    Complete,
    Partial,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationOrder {
    pub sequencing: Sequencing,
    pub short_circuit: ShortCircuit,
    pub unspecified_order: UnspecifiedOrder,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Sequencing {
    LanguageDefined,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CfgProfile {
    pub name: String,
    pub version: String,
    pub granularity: Granularity,
    pub evaluation_order: EvaluationOrder,
    pub exceptional_flow: ExceptionalFlow,
    pub exit_policy: String,
    pub unreachable_code: UnreachableCode,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CfgProducer {
    pub name: String,
    pub version: String,
    pub language: String,
    pub language_version: StringFact,
    pub configuration: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CfgSource {
    pub id: String,
    pub path: String,
    pub content_identity: StringFact,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CfgFunction {
    pub id: String,
    pub name: StringFact,
    pub source_span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CfgExtractionEvidence {
    pub status: ExtractionStatus,
    #[serde(default)]
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CfgOperation {
    pub id: String,
    pub original_kind: String,
    pub text: String,
    pub source_span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CfgNode {
    pub id: String,
    pub kind: CfgNodeKind,
    pub origin: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_kind: Option<CfgExitKind>,
    #[serde(default)]
    pub operations: Vec<CfgOperation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CfgEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: CfgEdgeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_role: Option<LoopRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CfgDocument {
    pub schema_version: String,
    pub profile: CfgProfile,
    pub producer: CfgProducer,
    pub sources: Vec<CfgSource>,
    pub function: CfgFunction,
    pub entry_node_id: String,
    pub exit_node_ids: Vec<String>,
    pub nodes: Vec<CfgNode>,
    pub edges: Vec<CfgEdge>,
    pub extraction_evidence: CfgExtractionEvidence,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: Extensions,
}

/// R0 construction policy: operation granularity with explicit branches.
pub fn r0_profile() -> CfgProfile {
    CfgProfile {
        name: R0_CFG_NAME.to_string(),
        version: R0_CFG_VERSION.to_string(),
        granularity: Granularity::Operation,
        evaluation_order: EvaluationOrder {
            sequencing: Sequencing::LanguageDefined,
            short_circuit: ShortCircuit::ExplicitBranches,
            unspecified_order: UnspecifiedOrder::Reject,
        },
        exceptional_flow: ExceptionalFlow::Omitted,
        exit_policy: "one normal exit; source returns only; no fabricated divergent fallthrough"
            .to_string(),
        unreachable_code: UnreachableCode::Preserve,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CfgIssue {
    pub code: &'static str,
    pub message: String,
}

/// Structural validation; semantic profile checks belong to a producer harness.
pub fn validate_cfg(doc: &CfgDocument) -> Vec<CfgIssue> {
    [
        version_issues(doc),
        id_issues(doc),
        presence_issues(doc),
        shape_issues(doc),
        endpoint_issues(doc),
        entry_issues(doc),
        metadata_issues(doc),
        edge_shape_issues(doc),
    ]
    .concat()
}

fn version_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    if doc.schema_version != crate::model::CFG_SCHEMA_VERSION {
        return vec![make_issue(
            "cfg.schema-version",
            "unexpected CFG schema_version",
        )];
    }
    Vec::new()
}

fn make_issue(code: &'static str, message: &str) -> CfgIssue {
    CfgIssue {
        code,
        message: message.to_string(),
    }
}

fn id_set(nodes: &[CfgNode]) -> BTreeSet<String> {
    nodes.iter().map(|n| n.id.clone()).collect()
}

fn id_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    [
        dup_node_ids(&doc.nodes),
        dup_edge_ids(&doc.edges),
        dup_source_ids(&doc.sources),
    ]
    .concat()
}

fn dup_node_ids(nodes: &[CfgNode]) -> Vec<CfgIssue> {
    dup_id_issues(nodes, |node| node.id.as_str())
}

fn dup_edge_ids(edges: &[CfgEdge]) -> Vec<CfgIssue> {
    dup_id_issues(edges, |edge| edge.id.as_str())
}

fn dup_source_ids(sources: &[CfgSource]) -> Vec<CfgIssue> {
    dup_id_issues(sources, |source| source.id.as_str())
}

fn dup_id_issues<T>(items: &[T], id_of: impl Fn(&T) -> &str) -> Vec<CfgIssue> {
    let mut seen = BTreeSet::new();

    items
        .iter()
        .map(id_of)
        .filter(|id| id.is_empty() || !seen.insert(id.to_string()))
        .map(|_| make_issue("cfg.duplicate-id", "bad or duplicate id"))
        .collect()
}

fn presence_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    let ids = id_set(&doc.nodes);

    [
        entry_presence(doc, &ids),
        exit_presence(doc, &ids),
        boundary_listing_issues(doc),
    ]
    .concat()
}

fn entry_presence(doc: &CfgDocument, ids: &BTreeSet<String>) -> Vec<CfgIssue> {
    if ids.contains(&doc.entry_node_id) {
        Vec::new()
    } else {
        vec![make_issue("cfg.entry", "entry node id is unknown")]
    }
}

fn exit_presence(doc: &CfgDocument, ids: &BTreeSet<String>) -> Vec<CfgIssue> {
    let mut issues = Vec::new();

    if doc.exit_node_ids.is_empty() {
        issues.push(make_issue("cfg.exit", "exit node list is empty"));
    }

    let mut seen = BTreeSet::new();

    for exit in &doc.exit_node_ids {
        if !seen.insert(exit.clone()) {
            issues.push(make_issue("cfg.exit", "duplicate exit node id"));
        }

        if !ids.contains(exit) {
            issues.push(make_issue("cfg.exit", "exit node id is unknown"));
        }
    }

    issues
}

/// Exactly one entry node; every exit-kind node must be listed and every
/// listed exit must be an exit-kind node.
fn boundary_listing_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    [
        entry_count_issues(doc),
        unlisted_exit_issues(doc),
        mislisted_exit_issues(doc),
    ]
    .concat()
}

fn entry_count_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    let entries: Vec<&CfgNode> = doc
        .nodes
        .iter()
        .filter(|node| node.kind == CfgNodeKind::Entry)
        .collect();

    if entries.len() != 1 {
        return vec![make_issue(
            "cfg.entry",
            "exactly one entry node is required",
        )];
    }

    if entries[0].id != doc.entry_node_id {
        return vec![make_issue(
            "cfg.entry",
            "entry node id must name the entry node",
        )];
    }

    Vec::new()
}

fn unlisted_exit_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    let exits: BTreeSet<&str> = doc.exit_node_ids.iter().map(String::as_str).collect();

    doc.nodes
        .iter()
        .filter(|node| node.kind == CfgNodeKind::Exit)
        .filter(|node| !exits.contains(node.id.as_str()))
        .map(|_| make_issue("cfg.exit", "exit node is not listed in exit_node_ids"))
        .collect()
}

fn mislisted_exit_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    doc.exit_node_ids
        .iter()
        .filter(|exit| {
            doc.nodes
                .iter()
                .find(|node| node.id == **exit)
                .is_some_and(|node| node.kind != CfgNodeKind::Exit)
        })
        .map(|_| make_issue("cfg.exit", "exit node id must name an exit node"))
        .collect()
}

fn shape_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    [
        node_shape_issues(doc),
        operation_id_issues(doc),
        granularity_issues(doc),
    ]
    .concat()
}

fn node_shape_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    doc.nodes.iter().flat_map(vertex_shape_issues).collect()
}

fn vertex_shape_issues(node: &CfgNode) -> Vec<CfgIssue> {
    let boundary = matches!(node.kind, CfgNodeKind::Entry | CfgNodeKind::Exit);

    operation_presence_issues(node, boundary)
        .into_iter()
        .chain(exit_kind_issue(node))
        .chain(origin_issue(node))
        .collect()
}

fn operation_presence_issues(node: &CfgNode, boundary: bool) -> Vec<CfgIssue> {
    if boundary && !node.operations.is_empty() {
        vec![make_issue(
            "cfg.boundary-operations",
            "boundary node carries operations",
        )]
    } else if !boundary && node.operations.is_empty() {
        vec![make_issue(
            "cfg.empty-operation",
            "non-boundary node lacks operations",
        )]
    } else {
        Vec::new()
    }
}

fn exit_kind_issue(node: &CfgNode) -> Option<CfgIssue> {
    match (node.kind, node.exit_kind) {
        (CfgNodeKind::Exit, None) => {
            Some(make_issue("cfg.exit-kind", "exit node lacks an exit kind"))
        }
        (CfgNodeKind::Exit, Some(_)) => None,
        (_, None) => None,
        _ => Some(make_issue(
            "cfg.exit-kind",
            "only exit nodes carry an exit kind",
        )),
    }
}

fn origin_issue(node: &CfgNode) -> Option<CfgIssue> {
    if node.origin.is_empty() {
        Some(make_issue("cfg.origin", "node origin must be nonempty"))
    } else {
        None
    }
}

/// Operation granularity requires exactly one operation per non-boundary node.
fn granularity_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    if doc.profile.granularity != Granularity::Operation {
        return Vec::new();
    }

    doc.nodes
        .iter()
        .filter(|node| {
            !matches!(node.kind, CfgNodeKind::Entry | CfgNodeKind::Exit)
                && node.operations.len() != 1
        })
        .map(|_| {
            make_issue(
                "cfg.granularity",
                "operation granularity requires one operation per non-boundary node",
            )
        })
        .collect()
}

fn operation_id_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    let mut seen = BTreeSet::new();

    doc.nodes
        .iter()
        .flat_map(|node| node.operations.iter())
        .filter(|op| op.id.is_empty() || !seen.insert(op.id.clone()))
        .map(|_| make_issue("cfg.duplicate-id", "duplicate operation id"))
        .collect()
}

fn endpoint_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    let ids = id_set(&doc.nodes);

    doc.edges
        .iter()
        .filter(|edge| !ids.contains(&edge.source) || !ids.contains(&edge.target))
        .map(|_| make_issue("cfg.dangling-edge", "edge names an unknown endpoint"))
        .collect()
}

/// Entry must have no incoming edges; reachability starts there.
fn entry_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    let mut issues: Vec<CfgIssue> = doc
        .edges
        .iter()
        .filter(|edge| edge.target == doc.entry_node_id)
        .map(|_| make_issue("cfg.entry-incoming", "entry has an incoming edge"))
        .collect();

    issues.extend(exit_outgoing_issues(doc));
    issues
}

fn exit_outgoing_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    let exits: BTreeSet<&str> = doc.exit_node_ids.iter().map(String::as_str).collect();

    doc.edges
        .iter()
        .filter(|edge| exits.contains(edge.source.as_str()))
        .map(|_| make_issue("cfg.exit-outgoing", "exit has an outgoing edge"))
        .collect()
}

fn metadata_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    [
        source_issues(doc),
        evidence_issues(doc),
        profile_issues(doc),
        function_fact_issues(doc),
    ]
    .concat()
}

fn source_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    if doc.sources.is_empty() {
        return vec![make_issue("cfg.sources", "source list is empty")];
    }

    doc.sources
        .iter()
        .flat_map(|source| {
            let mut issues = Vec::new();

            if source.path.is_empty() {
                issues.push(make_issue(
                    "cfg.source-path",
                    "source path must be nonempty",
                ));
            }

            if !source.content_identity.shape_ok() {
                issues.push(make_issue(
                    "cfg.fact-shape",
                    "malformed source content_identity fact",
                ));
            }

            issues
        })
        .collect()
}

fn evidence_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    let mut issues = Vec::new();
    let evidence = &doc.extraction_evidence;

    match evidence.status {
        ExtractionStatus::Complete => {}
        ExtractionStatus::Partial | ExtractionStatus::Unknown
            if evidence.limitations.is_empty() =>
        {
            issues.push(make_issue(
                "cfg.evidence",
                "partial or unknown evidence requires limitations",
            ))
        }
        _ => {}
    }

    for limitation in &evidence.limitations {
        if limitation.is_empty() {
            issues.push(make_issue(
                "cfg.evidence",
                "limitation entries must be nonempty",
            ));
        }
    }

    issues
}

fn profile_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    let mut issues = Vec::new();

    if doc.profile.name.is_empty() || doc.profile.version.is_empty() {
        issues.push(make_issue(
            "cfg.profile",
            "profile name and version must be nonempty",
        ));
    }

    if doc.profile.exit_policy.is_empty() {
        issues.push(make_issue(
            "cfg.exit-policy",
            "exit policy must be nonempty",
        ));
    }

    if doc.nodes.len() < 2 {
        issues.push(make_issue(
            "cfg.nodes",
            "node collection requires at least entry and exit",
        ));
    }

    issues
}

fn function_fact_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    let mut issues = Vec::new();

    if doc.function.id.is_empty() {
        issues.push(make_issue("cfg.function", "function id must be nonempty"));
    }

    if !doc.function.name.shape_ok() {
        issues.push(make_issue("cfg.fact-shape", "malformed function name fact"));
    }

    issues
}

fn edge_shape_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    doc.edges.iter().flat_map(edge_issues).collect()
}

fn edge_issues(edge: &CfgEdge) -> Vec<CfgIssue> {
    let mut issues = Vec::new();

    match (edge.kind, edge.case_value.as_deref()) {
        (CfgEdgeKind::Case, Some(value)) if !value.is_empty() => {}
        (CfgEdgeKind::Case, _) => {
            issues.push(make_issue(
                "cfg.edge-case",
                "case edges require a nonempty case_value",
            ));
        }
        (_, Some(_)) => {
            issues.push(make_issue(
                "cfg.edge-case",
                "only case edges carry a case_value",
            ));
        }
        _ => {}
    }

    if edge.reason.as_deref().is_some_and(str::is_empty) {
        issues.push(make_issue(
            "cfg.edge-reason",
            "edge reason must be nonempty",
        ));
    }

    issues
}

/// Vertices reachable from entry, following recorded CFG direction.
/// Each vertex is discovered and enqueued once, so parallel-edge fans stay linear.
pub fn reachable_vertices(doc: &CfgDocument) -> BTreeSet<String> {
    let mut seen = BTreeSet::from([doc.entry_node_id.clone()]);
    let mut frontier = vec![doc.entry_node_id.clone()];

    while !frontier.is_empty() {
        let fresh = fresh_successors(doc, &frontier, &mut seen);
        frontier = fresh;
    }

    seen
}

fn fresh_successors(
    doc: &CfgDocument,
    frontier: &[String],
    seen: &mut BTreeSet<String>,
) -> Vec<String> {
    let mut fresh = Vec::new();

    for current in frontier {
        for edge in doc.edges.iter().filter(|edge| edge.source == *current) {
            if seen.insert(edge.target.clone()) {
                fresh.push(edge.target.clone());
            }
        }
    }

    fresh
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationMapEntry {
    pub cfg_node_id: String,
    pub cfg_operation_id: String,
    pub pdg_node_id: String,
}

/// Read the shared operation-map extension from either document's extensions.
pub fn read_operation_map(extensions: &Extensions) -> Result<Vec<OperationMapEntry>, String> {
    let value = extensions
        .get(OPERATION_MAP_KEY)
        .ok_or_else(missing_map_msg)?;
    let triples = value
        .as_array()
        .ok_or_else(|| "operation-map is not an array".to_string())?;
    triples.iter().map(read_triple).collect()
}

fn missing_map_msg() -> String {
    format!("missing extension {OPERATION_MAP_KEY}")
}

fn read_triple(value: &serde_json::Value) -> Result<OperationMapEntry, String> {
    if !value.is_object() {
        return Err("operation-map triple must be an object".to_string());
    }

    let get = |field: &str| {
        value
            .get(field)
            .and_then(|v| v.as_str())
            .filter(|text| !text.is_empty())
            .map(str::to_string)
            .ok_or_else(|| format!("operation-map triple lacks {field}"))
    };

    let entry = OperationMapEntry {
        cfg_node_id: get("cfg_node_id")?,
        cfg_operation_id: get("cfg_operation_id")?,
        pdg_node_id: get("pdg_node_id")?,
    };

    reject_extra_triple_fields(value)?;

    Ok(entry)
}

fn reject_extra_triple_fields(value: &serde_json::Value) -> Result<(), String> {
    let allowed = ["cfg_node_id", "cfg_operation_id", "pdg_node_id"];

    value
        .as_object()
        .expect("triple is an object")
        .keys()
        .find(|key| !allowed.contains(&key.as_str()))
        .map_or(Ok(()), |key| {
            Err(format!("operation-map triple has forbidden field {key:?}"))
        })
}

/// Check cross-document correspondence against externally identified pairs.
///
/// Each reachable or unreachable semantic operation has exactly one triple,
/// one non-boundary CFG vertex, one CFG operation, and one PDG operation node.
/// No two distinct operations share a CFG vertex in R0. Triples must be
/// duplicate-free and one-to-one on both operation and PDG sides, and every
/// non-boundary CFG operation must be covered exactly once.
pub fn validate_correspondence(
    map: &[OperationMapEntry],
    cfg: &CfgDocument,
    pdg_node_ids: &BTreeSet<String>,
) -> Vec<CfgIssue> {
    let cfg_ops = cfg_operation_index(cfg);
    let endpoint = map
        .iter()
        .flat_map(|entry| triple_endpoint_issues(entry, &cfg_ops, pdg_node_ids))
        .collect::<Vec<_>>();
    let sharing = sharing_issues(map);
    let duplicates = duplicate_triple_issues(map);
    let coverage = coverage_issues(map, cfg);

    [endpoint, sharing, duplicates, coverage].concat()
}

fn cfg_operation_index(cfg: &CfgDocument) -> BTreeMap<String, (String, bool)> {
    let mut index = BTreeMap::new();

    for node in &cfg.nodes {
        let boundary = matches!(node.kind, CfgNodeKind::Entry | CfgNodeKind::Exit);

        for op in &node.operations {
            index.insert(op.id.clone(), (node.id.clone(), boundary));
        }
    }

    index
}

fn triple_endpoint_issues(
    entry: &OperationMapEntry,
    cfg_ops: &BTreeMap<String, (String, bool)>,
    pdg_node_ids: &BTreeSet<String>,
) -> Vec<CfgIssue> {
    [
        operation_issue(entry, cfg_ops),
        node_issue(entry, pdg_node_ids),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn operation_issue(
    entry: &OperationMapEntry,
    cfg_ops: &BTreeMap<String, (String, bool)>,
) -> Option<CfgIssue> {
    match cfg_ops.get(&entry.cfg_operation_id) {
        Some((vertex, false)) if *vertex == entry.cfg_node_id => None,
        _ => Some(make_issue(
            "cfg.operation-map",
            "triple names an unknown or boundary CFG operation",
        )),
    }
}

fn node_issue(entry: &OperationMapEntry, pdg_node_ids: &BTreeSet<String>) -> Option<CfgIssue> {
    if pdg_node_ids.contains(&entry.pdg_node_id) {
        None
    } else {
        Some(make_issue(
            "cfg.operation-map",
            "triple names an unknown PDG node",
        ))
    }
}

/// No two distinct operations share one CFG vertex in R0.
fn sharing_issues(map: &[OperationMapEntry]) -> Vec<CfgIssue> {
    let mut ordered: Vec<&OperationMapEntry> = map.iter().collect();
    ordered.sort_by_key(|entry| entry.cfg_node_id.clone());

    ordered
        .chunk_by(|a, b| a.cfg_node_id == b.cfg_node_id)
        .filter(|chunk| shared_vertex(chunk))
        .map(|_| make_issue("cfg.operation-map", "two operations share one CFG vertex"))
        .collect()
}

fn shared_vertex(chunk: &[&OperationMapEntry]) -> bool {
    chunk
        .iter()
        .map(|entry| entry.cfg_operation_id.as_str())
        .collect::<BTreeSet<_>>()
        .len()
        > 1
}

/// Duplicate triples and duplicate mappings on either side are forbidden:
/// the map must be one-to-one between CFG operations and PDG nodes.
fn duplicate_triple_issues(map: &[OperationMapEntry]) -> Vec<CfgIssue> {
    [
        repeated_triple_issues(map),
        repeated_operation_issues(map),
        repeated_pdg_issues(map),
    ]
    .concat()
}

fn repeated_triple_issues(map: &[OperationMapEntry]) -> Vec<CfgIssue> {
    repeat_issues(
        map.iter().map(|entry| {
            (
                entry.cfg_node_id.clone(),
                entry.cfg_operation_id.clone(),
                entry.pdg_node_id.clone(),
            )
        }),
        "duplicate operation-map triple",
    )
}

fn repeated_operation_issues(map: &[OperationMapEntry]) -> Vec<CfgIssue> {
    repeat_issues(
        map.iter().map(|entry| entry.cfg_operation_id.clone()),
        "CFG operation mapped more than once",
    )
}

fn repeated_pdg_issues(map: &[OperationMapEntry]) -> Vec<CfgIssue> {
    repeat_issues(
        map.iter().map(|entry| entry.pdg_node_id.clone()),
        "PDG node mapped more than once",
    )
}

fn repeat_issues<K: Ord + Clone>(
    keys: impl Iterator<Item = K>,
    message: &'static str,
) -> Vec<CfgIssue> {
    let mut seen = BTreeSet::new();
    let repeats = keys.filter(|key| !seen.insert(key.clone())).count();

    (0..repeats)
        .map(|_| make_issue("cfg.operation-map-duplicate", message))
        .collect()
}

/// Every non-boundary CFG operation must be covered exactly once.
fn coverage_issues(map: &[OperationMapEntry], cfg: &CfgDocument) -> Vec<CfgIssue> {
    let covered: BTreeSet<&str> = map
        .iter()
        .map(|entry| entry.cfg_operation_id.as_str())
        .collect();
    let mut issues = Vec::new();

    for node in &cfg.nodes {
        if matches!(node.kind, CfgNodeKind::Entry | CfgNodeKind::Exit) {
            continue;
        }

        for op in &node.operations {
            if !covered.contains(op.id.as_str()) {
                issues.push(make_issue(
                    "cfg.operation-map-incomplete",
                    "non-boundary CFG operation lacks an operation-map triple",
                ));
            }
        }
    }

    issues
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairedGraph {
    pub cfg_document_key: String,
    pub pdg_document_key: String,
}

/// Read the paired-graph link extension identifying the document pair.
pub fn read_paired_graph(extensions: &Extensions) -> Result<PairedGraph, String> {
    let value = extensions
        .get(PAIRED_GRAPH_KEY)
        .ok_or_else(|| format!("missing extension {PAIRED_GRAPH_KEY}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "paired-graph is not an object".to_string())?;
    let get = |field: &str| {
        object
            .get(field)
            .and_then(|v| v.as_str())
            .filter(|text| !text.is_empty())
            .map(str::to_string)
            .ok_or_else(|| format!("paired-graph lacks {field}"))
    };
    let paired = PairedGraph {
        cfg_document_key: get("cfg_document_key")?,
        pdg_document_key: get("pdg_document_key")?,
    };

    if object.len() != 2 {
        return Err("paired-graph has forbidden fields".to_string());
    }

    Ok(paired)
}

/// Both documents must carry the identical operation-map and agree on the
/// paired-graph link. Comparison is on parsed triples, never on raw text.
pub fn validate_paired_agreement(
    pdg_extensions: &Extensions,
    cfg_extensions: &Extensions,
) -> Vec<CfgIssue> {
    [
        operation_map_agreement(pdg_extensions, cfg_extensions),
        paired_graph_agreement(pdg_extensions, cfg_extensions),
    ]
    .concat()
}

fn operation_map_agreement(
    pdg_extensions: &Extensions,
    cfg_extensions: &Extensions,
) -> Vec<CfgIssue> {
    match (
        read_operation_map(pdg_extensions),
        read_operation_map(cfg_extensions),
    ) {
        (Ok(pdg_map), Ok(cfg_map)) => {
            if sorted_triples(&pdg_map) != sorted_triples(&cfg_map) {
                return vec![make_issue(
                    "cfg.paired-operation-map",
                    "paired documents disagree on the operation map",
                )];
            }

            Vec::new()
        }
        _ => vec![make_issue(
            "cfg.paired-operation-map",
            "both paired documents must carry the operation map",
        )],
    }
}

fn paired_graph_agreement(
    pdg_extensions: &Extensions,
    cfg_extensions: &Extensions,
) -> Vec<CfgIssue> {
    match (
        read_paired_graph(pdg_extensions),
        read_paired_graph(cfg_extensions),
    ) {
        (Ok(pdg_link), Ok(cfg_link)) => {
            if pdg_link != cfg_link {
                return vec![make_issue(
                    "cfg.paired-graph",
                    "paired documents disagree on the paired-graph link",
                )];
            }

            Vec::new()
        }
        _ => vec![make_issue(
            "cfg.paired-graph",
            "both paired documents must carry the paired-graph link",
        )],
    }
}

fn sorted_triples(map: &[OperationMapEntry]) -> Vec<(String, String, String)> {
    let mut triples: Vec<(String, String, String)> = map
        .iter()
        .map(|entry| {
            (
                entry.cfg_node_id.clone(),
                entry.cfg_operation_id.clone(),
                entry.pdg_node_id.clone(),
            )
        })
        .collect();
    triples.sort();
    triples
}
