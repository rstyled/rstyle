//! Directed, dependency-family-aware synchronous Weisfeiler-Lehman.
//!
//! Round zero holds the five initial labels. Each later round forms
//! `(previous_label, sorted incoming, sorted outgoing)` tuples with separate
//! in/out multisets, family order `data < control < execution`, preserved
//! multiplicity, and self-edges in both lists. Tuples intern in one shared
//! equality-preserving context across both graphs: equal tuples share a
//! label, and nothing hash-only decides equality.

use crate::{
    compact::{MatchingGraph, NodeId, ordered_nodes},
    graph::{ProjectedGraph, family_rank},
};
use std::collections::BTreeMap;
mod key;
use key::WlTuple;

/// Maximum finite shortest-path distance over reachable distinct ordered
/// vertex pairs, following recorded direction. Unreachable pairs are
/// ignored; graphs without one return 0. Parallel edges and lone
/// self-edges never lengthen a distance. Each vertex is discovered and
/// enqueued at most once per source BFS, so parallel-edge fans stay linear.
pub fn diameter(graph: &ProjectedGraph) -> usize {
    diameter_compact(&MatchingGraph::build(graph).expect("validated matching graph"))
}

pub(crate) fn diameter_compact(graph: &MatchingGraph) -> usize {
    graph
        .nodes()
        .map(|(node, _)| farthest_from(graph, node))
        .max()
        .unwrap_or(0)
}

fn farthest_from(graph: &MatchingGraph, start: NodeId) -> usize {
    let mut search = Search::new(graph.node_count(), start);
    while search.advance(graph) {}
    search.best
}

struct Search {
    seen: Vec<bool>,
    queue: Vec<(NodeId, usize)>,
    cursor: usize,
    best: usize,
}

impl Search {
    fn new(nodes: usize, start: NodeId) -> Self {
        let mut seen = vec![false; nodes];
        seen[start.index()] = true;
        Self {
            seen,
            queue: vec![(start, 0)],
            cursor: 0,
            best: 0,
        }
    }

    fn advance(&mut self, graph: &MatchingGraph) -> bool {
        let Some(&(current, distance)) = self.queue.get(self.cursor) else {
            return false;
        };
        self.cursor += 1;
        for (target, _) in graph.neighbors(current, false) {
            self.discover(target, distance + 1);
        }
        true
    }

    fn discover(&mut self, target: NodeId, distance: usize) {
        if !self.seen[target.index()] {
            self.seen[target.index()] = true;
            self.best = self.best.max(distance);
            self.queue.push((target, distance));
        }
    }
}

/// Pair depth: diameter of the fewer-node graph; on equal counts the
/// smaller diameter. Exactly `h` refinements keep rounds `0..=h`.
pub fn pair_depth(nodes_a: usize, diameter_a: usize, nodes_b: usize, diameter_b: usize) -> usize {
    if nodes_a == nodes_b {
        diameter_a.min(diameter_b)
    } else if nodes_a < nodes_b {
        diameter_a
    } else {
        diameter_b
    }
}

/// Positive integer weights `w_r = h + 1 - r` for rounds `0..=h`.
pub fn weights(h: usize) -> Vec<u64> {
    (0..=h).map(|r| (h + 1 - r) as u64).collect()
}

pub struct WlRefinement {
    /// One `(histogram_a, histogram_b)` per round `0..=h`, keyed by the
    /// shared interned label id.
    pub rounds: Vec<(BTreeMap<u64, u64>, BTreeMap<u64, u64>)>,
    pub interned_labels: usize,
}

/// Refine both graphs together for exactly `h` synchronous rounds.
pub fn refine(a: &ProjectedGraph, b: &ProjectedGraph, h: usize) -> WlRefinement {
    let a = MatchingGraph::build(a).expect("validated matching graph");
    let b = MatchingGraph::build(b).expect("validated matching graph");
    refine_compact(&a, &b, h)
}

pub(crate) fn refine_compact(a: &MatchingGraph, b: &MatchingGraph, h: usize) -> WlRefinement {
    refine_observed(a, b, h, |_, _| {})
}

/// Observe the exact synchronous labels used to form each scoring histogram.
pub(crate) fn refine_observed(
    a: &MatchingGraph,
    b: &MatchingGraph,
    h: usize,
    mut observe: impl FnMut(&[u64], &[u64]),
) -> WlRefinement {
    let mut state = seed_both(a, b);
    observe(&state.labels_a, &state.labels_b);
    for _ in 0..h {
        state = advance(a, b, state);
        observe(&state.labels_a, &state.labels_b);
    }

    WlRefinement {
        rounds: state.rounds,
        interned_labels: state.interner.len(),
    }
}

/// Node-to-shared-label conformance evidence using the scoring transitions.
/// Numeric labels are local to this pair; their equality defines each partition.
pub fn trace_partitions(a: &ProjectedGraph, b: &ProjectedGraph, h: usize) -> Vec<LabelPartition> {
    let compact_a = MatchingGraph::build(a).expect("validated matching graph");
    let compact_b = MatchingGraph::build(b).expect("validated matching graph");

    let first: Vec<_> = ordered_nodes(a)
        .iter()
        .map(|node| node.id.clone())
        .collect();
    let second: Vec<_> = ordered_nodes(b)
        .iter()
        .map(|node| node.id.clone())
        .collect();
    let names = LabelNames {
        first: &first,
        second: &second,
    };

    let mut partitions = Vec::new();
    refine_observed(&compact_a, &compact_b, h, |first, second| {
        partitions.push(names.partition(first, second));
    });
    partitions
}

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct LabelPartition {
    pub first: BTreeMap<String, u64>,
    pub second: BTreeMap<String, u64>,
}

pub(crate) struct LabelNames<'a> {
    pub first: &'a [String],
    pub second: &'a [String],
}

impl LabelNames<'_> {
    pub fn partition(&self, first: &[u64], second: &[u64]) -> LabelPartition {
        LabelPartition {
            first: readable_labels(self.first, first),
            second: readable_labels(self.second, second),
        }
    }
}

fn readable_labels(names: &[String], labels: &[u64]) -> BTreeMap<String, u64> {
    names.iter().cloned().zip(labels.iter().copied()).collect()
}

struct RefineState {
    labels_a: Vec<u64>,
    labels_b: Vec<u64>,
    interner: BTreeMap<WlTuple, u64>,
    scratch_a: Vec<u64>,
    scratch_b: Vec<u64>,
    rounds: Vec<(BTreeMap<u64, u64>, BTreeMap<u64, u64>)>,
}

fn seed_both(a: &MatchingGraph, b: &MatchingGraph) -> RefineState {
    let (labels_a, interner) = seed_labels(a, BTreeMap::new());
    let (labels_b, interner) = seed_labels(b, interner);
    let rounds = vec![histograms(&labels_a, &labels_b)];

    RefineState {
        labels_a,
        labels_b,
        interner,
        scratch_a: Vec::new(),
        scratch_b: Vec::new(),
        rounds,
    }
}

fn advance(a: &MatchingGraph, b: &MatchingGraph, state: RefineState) -> RefineState {
    let RefineState {
        labels_a,
        labels_b,
        interner,
        scratch_a,
        scratch_b,
        mut rounds,
    } = state;
    let (next_a, interner) = step(a, &labels_a, scratch_a, interner);
    let (next_b, interner) = step(b, &labels_b, scratch_b, interner);
    rounds.push(histograms(&next_a, &next_b));

    RefineState {
        labels_a: next_a,
        labels_b: next_b,
        scratch_a: labels_a,
        scratch_b: labels_b,
        interner,
        rounds,
    }
}

fn intern(tuple: WlTuple, mut interner: BTreeMap<WlTuple, u64>) -> (u64, BTreeMap<WlTuple, u64>) {
    let id = interner.len() as u64;
    let id = *interner.entry(tuple).or_insert(id);
    (id, interner)
}

fn seed_labels(
    graph: &MatchingGraph,
    interner: BTreeMap<WlTuple, u64>,
) -> (Vec<u64>, BTreeMap<WlTuple, u64>) {
    let mut labeled = Vec::with_capacity(graph.node_count());
    let mut table = interner;
    for (_, label) in graph.nodes() {
        let tuple = WlTuple {
            label: u64::from(label as u8),
            incoming: Vec::new(),
            outgoing: Vec::new(),
        };
        let (interned, next) = intern(tuple, table);
        table = next;
        labeled.push(interned);
    }
    (labeled, table)
}

fn step(
    graph: &MatchingGraph,
    previous: &[u64],
    mut labeled: Vec<u64>,
    interner: BTreeMap<WlTuple, u64>,
) -> (Vec<u64>, BTreeMap<WlTuple, u64>) {
    labeled.clear();
    labeled.reserve(graph.node_count());

    let mut table = interner;
    for (id, _) in graph.nodes() {
        let tuple = WlTuple {
            label: previous[id.index()],
            incoming: neighborhood(graph, id, previous, true),
            outgoing: neighborhood(graph, id, previous, false),
        };
        let (interned, next) = intern(tuple, table);
        table = next;
        labeled.push(interned);
    }
    (labeled, table)
}

fn neighborhood(
    graph: &MatchingGraph,
    id: NodeId,
    previous: &[u64],
    incoming: bool,
) -> Vec<(u8, u64)> {
    let mut entries: Vec<_> = graph
        .neighbors(id, incoming)
        .map(|(neighbor, family)| (family_rank(family), previous[neighbor.index()]))
        .collect();
    entries.sort();
    entries
}

fn histograms(a: &[u64], b: &[u64]) -> (BTreeMap<u64, u64>, BTreeMap<u64, u64>) {
    (count(a.iter()), count(b.iter()))
}

fn count<'a>(labels: impl Iterator<Item = &'a u64>) -> BTreeMap<u64, u64> {
    let mut out = BTreeMap::new();
    for label in labels {
        *out.entry(*label).or_default() += 1;
    }
    out
}

#[cfg(test)]
mod buffer_tests {
    use super::*;
    use crate::graph::{CcLabel, ProjectedNode};

    #[test]
    fn refinement_rotates_allocations_without_overwriting_previous_labels() {
        let projected = ProjectedGraph {
            nodes: vec![ProjectedNode {
                id: "node".into(),
                label: CcLabel::State,
            }],
            edges: Vec::new(),
        };
        let graph = MatchingGraph::build(&projected).unwrap();
        let initial = seed_both(&graph, &graph);
        let first_buffers = (initial.labels_a.as_ptr(), initial.labels_b.as_ptr());
        let first = advance(&graph, &graph, initial);
        let second_buffers = (first.labels_a.as_ptr(), first.labels_b.as_ptr());
        assert_eq!(
            (first.scratch_a.as_ptr(), first.scratch_b.as_ptr()),
            first_buffers
        );
        let second = advance(&graph, &graph, first);
        assert_eq!(
            (second.labels_a.as_ptr(), second.labels_b.as_ptr()),
            first_buffers
        );
        assert_eq!(
            (second.scratch_a.as_ptr(), second.scratch_b.as_ptr()),
            second_buffers
        );

        let expected = refine(&projected, &projected, 2);
        assert_eq!(second.rounds, expected.rounds);
        assert_eq!(second.interner.len(), expected.interned_labels);
        assert_eq!(second.labels_a, second.labels_b);
        assert_ne!(second.labels_a, second.scratch_a);
    }
}
