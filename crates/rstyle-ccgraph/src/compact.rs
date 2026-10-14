//! Immutable matching storage. Handles are local to one graph, never identities.
use crate::graph::{CcLabel, ProjectedGraph};
use rstyle_pdg::EdgeFamily;
use std::collections::BTreeMap;

/// Private handles cannot escape into another graph or a persistent contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NodeId(u32);

#[derive(Clone, Copy)]
struct EdgeId(u32);

struct Edge {
    source: NodeId,
    target: NodeId,
    family: EdgeFamily,
}

struct Adjacency {
    offsets: Vec<u32>,
    occurrences: Vec<EdgeId>,
}

impl Adjacency {
    fn build(nodes: usize, edges: &[Edge], endpoint: fn(&Edge) -> NodeId) -> Result<Self, String> {
        Self::from_buckets(edge_buckets(nodes, edges, endpoint)?, edges.len())
    }

    fn from_buckets(buckets: Vec<Vec<EdgeId>>, count: usize) -> Result<Self, String> {
        let mut offsets = vec![0];
        let mut occurrences = Vec::with_capacity(count);
        for bucket in buckets {
            occurrences.extend(bucket);
            offsets.push(portable(occurrences.len())?);
        }
        Ok(Self {
            offsets,
            occurrences,
        })
    }

    fn edges(&self, node: NodeId) -> &[EdgeId] {
        let index = node.index();
        &self.occurrences[self.offsets[index] as usize..self.offsets[index + 1] as usize]
    }
}

fn edge_buckets(
    nodes: usize,
    edges: &[Edge],
    endpoint: fn(&Edge) -> NodeId,
) -> Result<Vec<Vec<EdgeId>>, String> {
    let mut buckets = vec![Vec::new(); nodes];
    for (index, edge) in edges.iter().enumerate() {
        buckets[endpoint(edge).index()].push(EdgeId(portable(index)?));
    }
    Ok(buckets)
}

impl NodeId {
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

fn portable(index: usize) -> Result<u32, String> {
    u32::try_from(index).map_err(|_| "Matching graph exceeds u32 storage budget".into())
}

/// Nodes are numbered in readable-name order once at construction. Numbering
/// survives this immutable graph's lifetime, not arbitrary source edits. No
/// string endpoint or name is retained by the computational representation.
pub(crate) struct MatchingGraph {
    labels: Vec<CcLabel>,
    edges: Vec<Edge>,
    incoming: Adjacency,
    outgoing: Adjacency,
}

impl MatchingGraph {
    pub(crate) fn build(graph: &ProjectedGraph) -> Result<Self, String> {
        portable(graph.nodes.len())?;
        portable(graph.edges.len())?;
        let nodes = ordered_nodes(graph);
        let ids = node_ids(&nodes)?;
        let edges = graph
            .edges
            .iter()
            .map(|edge| {
                Ok(Edge {
                    source: *ids
                        .get(edge.source.as_str())
                        .ok_or("Dangling matching source")?,
                    target: *ids
                        .get(edge.target.as_str())
                        .ok_or("Dangling matching target")?,
                    family: edge.family,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let incoming = Adjacency::build(nodes.len(), &edges, |edge| edge.target)?;
        let outgoing = Adjacency::build(nodes.len(), &edges, |edge| edge.source)?;

        Ok(Self {
            labels: nodes.iter().map(|node| node.label).collect(),
            edges,
            incoming,
            outgoing,
        })
    }

    pub(crate) fn node_count(&self) -> usize {
        self.labels.len()
    }

    pub(crate) fn nodes(&self) -> impl Iterator<Item = (NodeId, CcLabel)> + '_ {
        self.labels.iter().enumerate().map(|(index, label)| {
            (
                NodeId(u32::try_from(index).expect("validated node count")),
                *label,
            )
        })
    }

    pub(crate) fn neighbors(
        &self,
        node: NodeId,
        incoming: bool,
    ) -> impl Iterator<Item = (NodeId, EdgeFamily)> + '_ {
        let adjacency = if incoming {
            &self.incoming
        } else {
            &self.outgoing
        };
        adjacency.edges(node).iter().map(move |id| {
            let edge = &self.edges[id.0 as usize];
            (
                if incoming { edge.source } else { edge.target },
                edge.family,
            )
        })
    }
}

pub(crate) fn ordered_nodes(graph: &ProjectedGraph) -> Vec<&crate::graph::ProjectedNode> {
    let mut nodes: Vec<_> = graph.nodes.iter().collect();
    nodes.sort_by(|a, b| a.id.cmp(&b.id));
    nodes
}

fn node_ids<'a>(
    nodes: &[&'a crate::graph::ProjectedNode],
) -> Result<BTreeMap<&'a str, NodeId>, String> {
    let mut ids = BTreeMap::new();
    for (index, node) in nodes.iter().enumerate() {
        if ids
            .insert(node.id.as_str(), NodeId(portable(index)?))
            .is_some()
        {
            return Err("Duplicate matching node identity".into());
        }
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{ProjectedEdge, ProjectedNode};

    fn graph() -> ProjectedGraph {
        ProjectedGraph {
            nodes: vec![
                ProjectedNode {
                    id: "z".into(),
                    label: CcLabel::State,
                },
                ProjectedNode {
                    id: "a".into(),
                    label: CcLabel::Decl,
                },
            ],
            edges: vec![
                ProjectedEdge {
                    source: "a".into(),
                    target: "z".into(),
                    family: EdgeFamily::Data,
                },
                ProjectedEdge {
                    source: "a".into(),
                    target: "z".into(),
                    family: EdgeFamily::Data,
                },
                ProjectedEdge {
                    source: "z".into(),
                    target: "z".into(),
                    family: EdgeFamily::Control,
                },
            ],
        }
    }

    #[test]
    fn adjacency_preserves_parallel_occurrences_and_self_edges() {
        let compact = MatchingGraph::build(&graph()).unwrap();
        assert_eq!(compact.labels, [CcLabel::Decl, CcLabel::State]);
        assert_eq!(compact.edges.len(), 3);
        let outgoing: Vec<_> = compact.neighbors(NodeId(0), false).collect();
        assert_eq!(outgoing, [(NodeId(1), EdgeFamily::Data); 2]);
        assert_eq!(compact.neighbors(NodeId(1), true).count(), 3);
        assert_eq!(compact.neighbors(NodeId(1), false).count(), 1);
        assert_eq!(crate::wl::diameter_compact(&compact), 1);
    }

    #[test]
    fn invalid_endpoints_and_duplicate_nodes_are_rejected() {
        let mut dangling = graph();
        dangling.edges[0].source = "missing".into();
        assert!(MatchingGraph::build(&dangling).is_err());
        dangling.edges[0].source = "a".into();
        dangling.edges[0].target = "missing".into();
        assert!(MatchingGraph::build(&dangling).is_err());
        let mut duplicate = graph();
        duplicate.nodes[1].id = "z".into();
        assert!(MatchingGraph::build(&duplicate).is_err());
        assert_eq!(portable(usize::MAX).is_err(), usize::BITS > 32);
    }

    #[test]
    fn storage_order_does_not_change_partitions_or_histograms() {
        let original = graph();
        let mut reordered = original.clone();
        reordered.nodes.reverse();
        reordered.edges.reverse();
        let before = crate::wl::refine(&original, &original, 3);
        let after = crate::wl::refine(&original, &reordered, 3);
        assert_eq!(before.rounds, after.rounds);
        let trace = crate::wl::trace_partitions(&original, &reordered, 3);
        assert!(
            trace
                .iter()
                .all(|partition| partition.first == partition.second)
        );
    }
}
