//! Directed, dependency-family-aware synchronous Weisfeiler-Lehman.
//!
//! Round zero holds the five initial labels. Each later round forms
//! `(previous_label, sorted incoming, sorted outgoing)` tuples with separate
//! in/out multisets, family order `data < control < execution`, preserved
//! multiplicity, and self-edges in both lists. Tuples intern in one shared
//! equality-preserving context across both graphs: equal tuples share a
//! label, and nothing hash-only decides equality.

use crate::graph::{ProjectedGraph, family_rank};
use std::collections::{BTreeMap, BTreeSet};

/// Maximum finite shortest-path distance over reachable distinct ordered
/// vertex pairs, following recorded direction. Unreachable pairs are
/// ignored; graphs without one return 0. Parallel edges and lone
/// self-edges never lengthen a distance. Each vertex is discovered and
/// enqueued at most once per source BFS, so parallel-edge fans stay linear.
pub fn diameter(graph: &ProjectedGraph) -> usize {
    let mut best = 0;
    for start in graph.ordered_ids() {
        best = best.max(farthest_from(graph, &start));
    }
    best
}

fn farthest_from(graph: &ProjectedGraph, start: &str) -> usize {
    let mut seen = BTreeSet::from([start.to_string()]);
    let mut frontier = vec![start.to_string()];
    let mut best = 0;

    while !frontier.is_empty() {
        let fresh = fresh_targets(graph, &frontier, &mut seen);
        best += usize::from(!fresh.is_empty());
        frontier = fresh;
    }

    best
}

fn fresh_targets(
    graph: &ProjectedGraph,
    frontier: &[String],
    seen: &mut BTreeSet<String>,
) -> Vec<String> {
    let mut fresh = Vec::new();
    for current in frontier {
        for edge in graph.edges.iter().filter(|edge| edge.source == *current) {
            // Discover-once: insert at discovery so parallel edges sharing one
            // target enqueue that target a single time.
            if seen.insert(edge.target.clone()) {
                fresh.push(edge.target.clone());
            }
        }
    }
    fresh
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct WlTuple {
    label: u64,
    incoming: Vec<(u8, u64)>,
    outgoing: Vec<(u8, u64)>,
}

pub struct WlRefinement {
    /// One `(histogram_a, histogram_b)` per round `0..=h`, keyed by the
    /// shared interned label id.
    pub rounds: Vec<(BTreeMap<u64, u64>, BTreeMap<u64, u64>)>,
    pub interned_labels: usize,
}

/// Refine both graphs together for exactly `h` synchronous rounds.
pub fn refine(a: &ProjectedGraph, b: &ProjectedGraph, h: usize) -> WlRefinement {
    let seeded = seed_both(a, b);
    let finished = (0..h).fold(seeded, |state, _| advance(a, b, state));

    WlRefinement {
        rounds: finished.rounds,
        interned_labels: finished.interner.len(),
    }
}

/// Node-to-shared-label conformance evidence using the scoring transitions.
/// Numeric labels are local to this pair; their equality defines each partition.
pub fn trace_partitions(a: &ProjectedGraph, b: &ProjectedGraph, h: usize) -> Vec<LabelPartition> {
    let state = seed_both(a, b);
    let initial = label_partition(&state);
    let (_, partitions) = (0..h).fold((state, vec![initial]), |(state, mut partitions), _| {
        let next = advance(a, b, state);
        partitions.push(label_partition(&next));
        (next, partitions)
    });

    partitions
}

#[derive(serde::Serialize)]
pub struct LabelPartition {
    pub first: BTreeMap<String, u64>,
    pub second: BTreeMap<String, u64>,
}

fn label_partition(state: &RefineState) -> LabelPartition {
    LabelPartition {
        first: state.labels_a.clone(),
        second: state.labels_b.clone(),
    }
}

struct RefineState {
    labels_a: BTreeMap<String, u64>,
    labels_b: BTreeMap<String, u64>,
    interner: BTreeMap<WlTuple, u64>,
    rounds: Vec<(BTreeMap<u64, u64>, BTreeMap<u64, u64>)>,
}

fn seed_both(a: &ProjectedGraph, b: &ProjectedGraph) -> RefineState {
    let (labels_a, interner) = seed_labels(a, BTreeMap::new());
    let (labels_b, interner) = seed_labels(b, interner);
    let rounds = vec![histograms(&labels_a, &labels_b)];

    RefineState {
        labels_a,
        labels_b,
        interner,
        rounds,
    }
}

fn advance(a: &ProjectedGraph, b: &ProjectedGraph, state: RefineState) -> RefineState {
    let RefineState {
        labels_a,
        labels_b,
        interner,
        mut rounds,
    } = state;
    let (labels_a, interner) = step(a, &labels_a, interner);
    let (labels_b, interner) = step(b, &labels_b, interner);
    rounds.push(histograms(&labels_a, &labels_b));

    RefineState {
        labels_a,
        labels_b,
        interner,
        rounds,
    }
}

fn intern(tuple: WlTuple, mut interner: BTreeMap<WlTuple, u64>) -> (u64, BTreeMap<WlTuple, u64>) {
    if let Some(id) = interner.get(&tuple) {
        return (*id, interner);
    }
    let id = interner.len() as u64;
    interner.insert(tuple, id);
    (id, interner)
}

fn seed_labels(
    graph: &ProjectedGraph,
    interner: BTreeMap<WlTuple, u64>,
) -> (BTreeMap<String, u64>, BTreeMap<WlTuple, u64>) {
    let mut labeled = BTreeMap::new();
    let mut table = interner;
    for id in graph.ordered_ids() {
        let seed = u64::from(graph.label_of(&id).expect("ordered ids are graph nodes") as u8);
        let tuple = WlTuple {
            label: seed,
            incoming: Vec::new(),
            outgoing: Vec::new(),
        };
        let (interned, next) = intern(tuple, table);
        table = next;
        labeled.insert(id, interned);
    }
    (labeled, table)
}

fn step(
    graph: &ProjectedGraph,
    previous: &BTreeMap<String, u64>,
    interner: BTreeMap<WlTuple, u64>,
) -> (BTreeMap<String, u64>, BTreeMap<WlTuple, u64>) {
    let neighborhoods = collect_neighborhoods(graph, previous);
    let mut labeled = BTreeMap::new();
    let mut table = interner;
    for id in graph.ordered_ids() {
        let tuple = neighborhood_tuple(&id, previous, &neighborhoods);
        let (interned, next) = intern(tuple, table);
        table = next;
        labeled.insert(id, interned);
    }

    (labeled, table)
}

struct Neighborhoods {
    incoming: BTreeMap<String, Vec<(u8, u64)>>,
    outgoing: BTreeMap<String, Vec<(u8, u64)>>,
}

fn collect_neighborhoods(
    graph: &ProjectedGraph,
    previous: &BTreeMap<String, u64>,
) -> Neighborhoods {
    let mut incoming: BTreeMap<String, Vec<(u8, u64)>> = BTreeMap::new();
    let mut outgoing: BTreeMap<String, Vec<(u8, u64)>> = BTreeMap::new();
    for edge in &graph.edges {
        let rank = family_rank(edge.family);
        incoming
            .entry(edge.target.clone())
            .or_default()
            .push((rank, previous[&edge.source]));
        outgoing
            .entry(edge.source.clone())
            .or_default()
            .push((rank, previous[&edge.target]));
    }
    Neighborhoods { incoming, outgoing }
}

fn neighborhood_tuple(
    id: &str,
    previous: &BTreeMap<String, u64>,
    neighborhoods: &Neighborhoods,
) -> WlTuple {
    WlTuple {
        label: previous[id],
        incoming: sorted_view(neighborhoods.incoming.get(id)),
        outgoing: sorted_view(neighborhoods.outgoing.get(id)),
    }
}

fn sorted_view(entries: Option<&Vec<(u8, u64)>>) -> Vec<(u8, u64)> {
    let mut picked = entries.cloned().unwrap_or_default();
    picked.sort();
    picked
}

fn histograms(
    a: &BTreeMap<String, u64>,
    b: &BTreeMap<String, u64>,
) -> (BTreeMap<u64, u64>, BTreeMap<u64, u64>) {
    (count(a.values()), count(b.values()))
}

fn count<'a>(labels: impl Iterator<Item = &'a u64>) -> BTreeMap<u64, u64> {
    let mut out = BTreeMap::new();
    for label in labels {
        *out.entry(*label).or_default() += 1;
    }
    out
}
