//! Conservative matching projection with transformation provenance.
//!
//! The extracted graph is never modified. This module emits a separate
//! projected view plus a per-node/per-edge provenance record. No splicing,
//! no canonicalization, no read sharing: retained edges keep their direction,
//! family, and multiplicity.

use crate::graph::*;
use rstyle_pdg::NodeCategory;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectionFault {
    /// The function cannot be projected under this profile version.
    Ineligible { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeDisposition {
    Retained { reason: &'static str },
    Removed { reason: &'static str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EdgeDisposition {
    Retained { reason: &'static str },
    Dropped { reason: &'static str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeTrace {
    /// Index of the occurrence in the imported edge list; parallel edges on
    /// identical endpoints keep distinct indices.
    pub occurrence_index: usize,
    pub source: String,
    pub target: String,
    pub family: rstyle_pdg::EdgeFamily,
    /// Imported origin reference (edge id when supplied, else occurrence index).
    pub origin: String,
    pub disposition: EdgeDisposition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionTrace {
    pub nodes: Vec<(String, NodeDisposition)>,
    pub edges: Vec<EdgeTrace>,
    pub edges_retained: usize,
    pub edges_dropped: usize,
    pub dropped_for_unreachable: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionFacts {
    pub reference_count: u64,
    pub category_counts: BTreeMap<&'static str, u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Projection {
    pub graph: ProjectedGraph,
    pub trace: ProjectionTrace,
    pub facts: ProjectionFacts,
}

/// Project one imported function; entry/exit survive only as documented below.
pub fn project(func: &ImportedFunction) -> Result<Projection, ProjectionFault> {
    reject_admission_blockers(func)?;
    let kept = retained_ids(func)?;
    let graph = build_view(func, &kept)?;
    check_crossing_edges(func, &kept)?;

    let trace = trace_projection(func, &kept, &graph);
    let facts = projection_facts(func, &kept)?;

    Ok(Projection {
        graph,
        trace,
        facts,
    })
}

fn reject_admission_blockers(func: &ImportedFunction) -> Result<(), ProjectionFault> {
    for node in &func.nodes {
        if node.category == NodeCategory::MemorySummary {
            return Err(ineligible(format!("node {:?} is memory_summary", node.id)));
        }
        if node.category == NodeCategory::Other && node.registered_meaning.is_none() {
            return Err(ineligible(format!(
                "node {:?} is other without a registered meaning",
                node.id
            )));
        }
    }
    Ok(())
}

fn ineligible(reason: String) -> ProjectionFault {
    ProjectionFault::Ineligible { reason }
}

fn retained_ids(func: &ImportedFunction) -> Result<BTreeSet<String>, ProjectionFault> {
    let mut kept = BTreeSet::new();
    for node in &func.nodes {
        if keep_node(node)? {
            kept.insert(node.id.clone());
        }
    }
    Ok(kept)
}

fn keep_node(node: &ImportedNode) -> Result<bool, ProjectionFault> {
    if matches!(node.category, NodeCategory::Entry | NodeCategory::Exit) {
        return keep_boundary(node);
    }
    if !node.reachable {
        return Ok(keep_unreachable(node));
    }
    Ok(true)
}

/// Entry/exit nodes go only when administrative and free of value semantics.
fn keep_boundary(node: &ImportedNode) -> Result<bool, ProjectionFault> {
    if node.administrative && !node.defines_binding && !node.reads_or_writes {
        return Ok(false);
    }
    Err(ineligible(format!(
        "boundary node {:?} carries value semantics; explicit representation required",
        node.id
    )))
}

fn keep_unreachable(node: &ImportedNode) -> bool {
    match node.category {
        NodeCategory::FormalInput => true,
        NodeCategory::Merge => node.value_site_reachable,
        _ => false,
    }
}

fn build_view(
    func: &ImportedFunction,
    kept: &BTreeSet<String>,
) -> Result<ProjectedGraph, ProjectionFault> {
    let mut graph = ProjectedGraph::default();
    for node in &func.nodes {
        if kept.contains(&node.id) {
            graph.nodes.push(ProjectedNode {
                id: node.id.clone(),
                label: map_label(node)?,
            });
        }
    }
    for edge in &func.edges {
        if kept.contains(&edge.source) && kept.contains(&edge.target) {
            graph.edges.push(ProjectedEdge {
                source: edge.source.clone(),
                target: edge.target.clone(),
                family: edge.family,
            });
        }
    }
    Ok(graph)
}

fn map_label(node: &ImportedNode) -> Result<CcLabel, ProjectionFault> {
    initial_label(node.category)
        .ok_or_else(|| ineligible(format!("node {:?} has no matching label", node.id)))
}

/// A retained consumer fed by an unreachable-only producer hides a gap.
fn check_crossing_edges(
    func: &ImportedFunction,
    kept: &BTreeSet<String>,
) -> Result<(), ProjectionFault> {
    let removed_unreachable: BTreeSet<&str> = func
        .nodes
        .iter()
        .filter(|n| !kept.contains(&n.id) && !n.reachable && !is_boundary(n))
        .map(|n| n.id.as_str())
        .collect();
    for edge in &func.edges {
        if removed_unreachable.contains(edge.source.as_str()) && kept.contains(&edge.target) {
            return Err(ineligible(format!(
                "edge {:?}->{:?} crosses the reachability boundary",
                edge.source, edge.target
            )));
        }
    }
    Ok(())
}

fn is_boundary(node: &ImportedNode) -> bool {
    matches!(node.category, NodeCategory::Entry | NodeCategory::Exit)
}

fn trace_projection(
    func: &ImportedFunction,
    kept: &BTreeSet<String>,
    graph: &ProjectedGraph,
) -> ProjectionTrace {
    let nodes = func
        .nodes
        .iter()
        .map(|n| (n.id.clone(), disposition(n, kept)))
        .collect();

    let unreachable = unreachable_ids(func, kept);
    let edges = trace_edges(func, kept, &unreachable);
    let dropped_for_unreachable = count_unreachable_drops(&edges, &unreachable);

    ProjectionTrace {
        nodes,
        edges_retained: graph.edges.len(),
        edges_dropped: func.edges.len() - graph.edges.len(),
        dropped_for_unreachable,
        edges,
    }
}

fn unreachable_ids<'a>(func: &'a ImportedFunction, kept: &BTreeSet<String>) -> BTreeSet<&'a str> {
    func.nodes
        .iter()
        .filter(|n| !kept.contains(&n.id) && !n.reachable && !is_boundary(n))
        .map(|n| n.id.as_str())
        .collect()
}

fn trace_edges(
    func: &ImportedFunction,
    kept: &BTreeSet<String>,
    unreachable: &BTreeSet<&str>,
) -> Vec<EdgeTrace> {
    func.edges
        .iter()
        .enumerate()
        .map(|(index, e)| trace_one(index, e, kept, unreachable))
        .collect()
}

fn trace_one(
    index: usize,
    e: &ImportedEdge,
    kept: &BTreeSet<String>,
    unreachable: &BTreeSet<&str>,
) -> EdgeTrace {
    EdgeTrace {
        occurrence_index: index,
        source: e.source.clone(),
        target: e.target.clone(),
        family: e.family,
        origin: e.origin.clone(),
        disposition: edge_disposition(e, kept, unreachable),
    }
}

fn edge_disposition(
    e: &ImportedEdge,
    kept: &BTreeSet<String>,
    unreachable: &BTreeSet<&str>,
) -> EdgeDisposition {
    if kept.contains(&e.source) && kept.contains(&e.target) {
        return EdgeDisposition::Retained {
            reason: "retained-edge-occurrence",
        };
    }
    if unreachable.contains(e.source.as_str()) || unreachable.contains(e.target.as_str()) {
        return EdgeDisposition::Dropped {
            reason: "dropped-entry-unreachable",
        };
    }
    EdgeDisposition::Dropped {
        reason: "dropped-removed-endpoint",
    }
}

fn count_unreachable_drops(edges: &[EdgeTrace], unreachable: &BTreeSet<&str>) -> usize {
    edges
        .iter()
        .filter(|e| matches!(e.disposition, EdgeDisposition::Dropped { .. }))
        .filter(|e| {
            unreachable.contains(e.source.as_str()) || unreachable.contains(e.target.as_str())
        })
        .count()
}

fn disposition(node: &ImportedNode, kept: &BTreeSet<String>) -> NodeDisposition {
    if kept.contains(&node.id) {
        return NodeDisposition::Retained {
            reason: retained_reason(node),
        };
    }
    NodeDisposition::Removed {
        reason: removed_reason(node),
    }
}

fn retained_reason(node: &ImportedNode) -> &'static str {
    if node.category == NodeCategory::FormalInput {
        "retained-formal-input"
    } else if node.category == NodeCategory::Merge {
        "retained-required-merge"
    } else {
        "retained-reachable-operation"
    }
}

fn removed_reason(node: &ImportedNode) -> &'static str {
    if is_boundary(node) {
        "removed-administrative-boundary"
    } else {
        "removed-entry-unreachable"
    }
}

fn projection_facts(
    func: &ImportedFunction,
    kept: &BTreeSet<String>,
) -> Result<ProjectionFacts, ProjectionFault> {
    let recomputed = recomputed_reference_count(func, kept);
    match func.reference_count {
        None => Err(ineligible(
            "unknown reference count cannot be projected".to_string(),
        )),
        Some(known) if !func.reference_policy_ok || known != recomputed => Err(ineligible(
            format!("producer reference count {known} disagrees with recomputed {recomputed}"),
        )),
        Some(known) => Ok(ProjectionFacts {
            reference_count: known,
            category_counts: count_labels(func, kept)?,
        }),
    }
}

/// Distinct local/parameter symbols read by retained reads, including
/// decomposed compound reads. Repeated reads count once.
fn recomputed_reference_count(func: &ImportedFunction, kept: &BTreeSet<String>) -> u64 {
    let mut symbols = BTreeSet::new();
    for node in &func.nodes {
        if kept.contains(&node.id) && node.category == NodeCategory::Read {
            symbols.extend(node.read_symbols.iter().cloned());
        }
    }
    symbols.len() as u64
}

fn count_labels(
    func: &ImportedFunction,
    kept: &BTreeSet<String>,
) -> Result<BTreeMap<&'static str, u64>, ProjectionFault> {
    let mut counts: BTreeMap<&'static str, u64> = BTreeMap::new();
    for node in &func.nodes {
        if kept.contains(&node.id) {
            let label = map_label(node)?.token();
            *counts.entry(label).or_default() += 1;
        }
    }
    Ok(counts)
}
