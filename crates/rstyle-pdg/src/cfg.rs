//! CFG interchange model, reachability, and CFG/PDG correspondence.
//!
//! The matcher derives operation reachability from the paired CFG's entry,
//! never from array order. Correspondence triples live in the shared
//! `compat.rust.hir-r0:operation-map` extension on both documents.

use crate::model::{Extensions, ProfileId, Span};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const OPERATION_MAP_KEY: &str = "compat.rust.hir-r0:operation-map";
pub const PAIRED_GRAPH_KEY: &str = "compat.rust.hir-r0:paired-graph";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CfgNodeKind {
    Entry,
    Exit,
    Block,
    Condition,
    Transfer,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct CfgOperation {
    pub id: String,
    pub original_kind: String,
    pub text: String,
    pub source_span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct CfgNode {
    pub id: String,
    pub kind: CfgNodeKind,
    pub origin: String,
    #[serde(default)]
    pub exit_kind: Option<String>,
    #[serde(default)]
    pub operations: Vec<CfgOperation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct CfgEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct CfgDocument {
    pub schema_version: String,
    pub profile: ProfileId,
    pub entry_node_id: String,
    pub exit_node_ids: Vec<String>,
    pub nodes: Vec<CfgNode>,
    pub edges: Vec<CfgEdge>,
    #[serde(default)]
    pub extensions: Extensions,
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
    [dup_node_ids(&doc.nodes), dup_edge_ids(&doc.edges)].concat()
}

fn dup_node_ids(nodes: &[CfgNode]) -> Vec<CfgIssue> {
    dup_id_issues(nodes, |node| node.id.as_str())
}

fn dup_edge_ids(edges: &[CfgEdge]) -> Vec<CfgIssue> {
    dup_id_issues(edges, |edge| edge.id.as_str())
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
    [entry_presence(doc, &ids), exit_presence(doc, &ids)].concat()
}

fn entry_presence(doc: &CfgDocument, ids: &BTreeSet<String>) -> Vec<CfgIssue> {
    if ids.contains(&doc.entry_node_id) {
        Vec::new()
    } else {
        vec![make_issue("cfg.entry", "entry node id is unknown")]
    }
}

fn exit_presence(doc: &CfgDocument, ids: &BTreeSet<String>) -> Vec<CfgIssue> {
    doc.exit_node_ids
        .iter()
        .filter(|exit| !ids.contains(*exit))
        .map(|_| make_issue("cfg.exit", "exit node id is unknown"))
        .collect()
}

fn shape_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    [node_shape_issues(doc), operation_id_issues(doc)].concat()
}

fn node_shape_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    doc.nodes.iter().flat_map(vertex_shape_issues).collect()
}

fn vertex_shape_issues(node: &CfgNode) -> Vec<CfgIssue> {
    let boundary = matches!(node.kind, CfgNodeKind::Entry | CfgNodeKind::Exit);
    operation_presence_issues(node, boundary)
        .into_iter()
        .chain(exit_kind_issue(node))
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
    if node.kind == CfgNodeKind::Exit && node.exit_kind.is_none() {
        Some(make_issue("cfg.exit-kind", "exit node lacks an exit kind"))
    } else {
        None
    }
}

fn operation_id_issues(doc: &CfgDocument) -> Vec<CfgIssue> {
    let mut seen = BTreeSet::new();
    doc.nodes
        .iter()
        .flat_map(|node| node.operations.iter())
        .filter(|op| !seen.insert(op.id.clone()))
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
    doc.edges
        .iter()
        .filter(|edge| edge.target == doc.entry_node_id)
        .map(|_| make_issue("cfg.entry-incoming", "entry has an incoming edge"))
        .collect()
}

/// Vertices reachable from entry, following recorded CFG direction.
pub fn reachable_vertices(doc: &CfgDocument) -> BTreeSet<String> {
    let mut seen = BTreeSet::from([doc.entry_node_id.clone()]);
    let mut frontier = vec![doc.entry_node_id.clone()];

    while !frontier.is_empty() {
        let fresh = fresh_successors(doc, &frontier, &seen);
        seen.extend(fresh.iter().cloned());
        frontier = fresh;
    }

    seen
}

fn fresh_successors(
    doc: &CfgDocument,
    frontier: &[String],
    seen: &BTreeSet<String>,
) -> Vec<String> {
    frontier
        .iter()
        .flat_map(|current| doc.edges.iter().filter(move |edge| edge.source == *current))
        .map(|edge| edge.target.clone())
        .filter(|target| !seen.contains(target))
        .collect()
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
    let get = |field: &str| {
        value
            .get(field)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| format!("operation-map triple lacks {field}"))
    };
    Ok(OperationMapEntry {
        cfg_node_id: get("cfg_node_id")?,
        cfg_operation_id: get("cfg_operation_id")?,
        pdg_node_id: get("pdg_node_id")?,
    })
}

/// Check cross-document correspondence against externally identified pairs.
///
/// Each reachable or unreachable semantic operation has exactly one triple,
/// one non-boundary CFG vertex, one CFG operation, and one PDG operation node.
/// No two distinct operations share a CFG vertex in R0.
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

    [endpoint, sharing].concat()
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
