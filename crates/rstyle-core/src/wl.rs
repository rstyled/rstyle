//! Weisfeiler-Lehman style kernel over dependence graphs (CCGraph-inspired).
//!
//! Nodes start with a label (kind plus callee signature class), then repeatedly absorb the
//! sorted labels of their in- and out-neighbours, tagged with edge kind and operand position.
//! Similarity is a weighted histogram intersection over the refinement levels.
//! Hashing is FNV-1a so output is byte-identical across runs and platforms.

use crate::facts::{EdgeKind, NodeKind, Pdg, PdgNode};
use crate::shape::{class_of, normalize};
use std::collections::{BTreeMap, BTreeSet};

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const IN_SALT: u64 = 0x1f;
const OUT_SALT: u64 = 0x2e;

/// Refinement levels at most (level 1 is the raw labels).
pub const MAX_LEVELS: usize = 4;
/// Smallest graph worth comparing: nodes, and at least this many calls.
pub const MIN_NODES: usize = 4;
pub const MIN_CALLS: usize = 1;
/// The larger graph may have at most this many times the nodes of the smaller (CCGraph uses 10).
pub const MAX_SCALE: usize = 2;
/// Graphs above this size skip the exact diameter and use `MAX_LEVELS`.
const DIAMETER_LIMIT: usize = 400;

pub fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

/// Order-sensitive mix of two values (both are hashed, so `combine(a, b) != combine(b, a)`).
pub fn combine(acc: u64, value: u64) -> u64 {
    let bytes = acc.to_le_bytes().into_iter().chain(value.to_le_bytes());
    bytes.fold(FNV_OFFSET, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    })
}

/// Small summaries collected together during the existing call-count pass.
struct CallSummary {
    calls: usize,
    forwarding_shape: bool,
    opaque_closure: bool,
}

impl CallSummary {
    fn observe(self, node: &PdgNode) -> Self {
        let forwarding = match node.kind {
            NodeKind::Param | NodeKind::Return => true,
            NodeKind::Call => node.callee_id.as_ref().is_some_and(|id| !id.is_empty()),
            _ => false,
        };
        Self {
            calls: self.calls + usize::from(node.kind == NodeKind::Call),
            forwarding_shape: self.forwarding_shape && forwarding,
            opaque_closure: self.opaque_closure || node.kind == NodeKind::Closure,
        }
    }
}

/// Signature class of a call: normalized callee signature, or the callee id without one.
pub fn call_class(node: &PdgNode) -> String {
    match (&node.callee_sig, &node.callee_id) {
        (Some(sig), _) => normalize(sig).key,
        (None, Some(id)) => id.clone(),
        (None, None) => "?".into(),
    }
}

/// Label text of a node. Names (callee ids, fields, literal values) are deliberately absent.
pub fn label(node: &PdgNode) -> String {
    let field = |name: &Option<String>| name.clone().unwrap_or_default();
    match node.kind {
        NodeKind::Call => format!("call:{}", call_class(node)),
        NodeKind::Macro => format!("macro:{}", field(&node.macro_name)),
        NodeKind::Lit => format!("lit:{}", field(&node.lit_kind)),
        // `cmp:<` (canonical comparison): the operator after the colon is a hole, not part of the class.
        NodeKind::Op => format!("op:{}", node.detail.split(':').next().unwrap_or_default()),
        NodeKind::Param => format!(
            "param:{}",
            node.ty.as_ref().map_or_else(String::new, class_of)
        ),
        kind => format!("{kind:?}"),
    }
}

fn edge_tag(kind: EdgeKind, arg_pos: usize) -> u64 {
    combine(u64::from(kind == EdgeKind::Ctrl), arg_pos as u64)
}

/// A neighbour reached over an edge with this tag (kind and position).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nb {
    pub node: usize,
    pub tag: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Graph {
    pub labels: Vec<u64>,
    pub inc: Vec<Vec<Nb>>,
    pub out: Vec<Vec<Nb>>,
}

fn adjacency(n: usize, pdg: &Pdg, forward: bool) -> Vec<Vec<Nb>> {
    let mut lists = vec![Vec::new(); n];
    for e in pdg.edges.iter().filter(|e| e.from < n && e.to < n) {
        let (at, node) = if forward {
            (e.from, e.to)
        } else {
            (e.to, e.from)
        };
        lists[at].push(Nb {
            node,
            tag: edge_tag(e.kind, e.arg_pos),
        });
    }
    lists
}

pub fn graph(pdg: &Pdg) -> Graph {
    let n = pdg.nodes.len();
    Graph {
        labels: pdg
            .nodes
            .iter()
            .map(|node| fnv1a(label(node).as_bytes()))
            .collect(),
        inc: adjacency(n, pdg, false),
        out: adjacency(n, pdg, true),
    }
}

fn digest(own: u64, salt: u64, neighbours: &[Nb], prev: &[u64]) -> u64 {
    let mut parts: Vec<u64> = neighbours
        .iter()
        .map(|nb| combine(nb.tag, prev[nb.node]))
        .collect();
    parts.sort_unstable();
    parts.into_iter().fold(combine(own, salt), combine)
}

fn refine(g: &Graph, prev: &[u64]) -> Vec<u64> {
    (0..prev.len())
        .map(|v| {
            let inward = digest(prev[v], IN_SALT, &g.inc[v], prev);
            digest(inward, OUT_SALT, &g.out[v], prev)
        })
        .collect()
}

fn histogram(colors: &[u64]) -> Vec<(u64, usize)> {
    let counts = colors.iter().fold(BTreeMap::new(), |mut acc, color| {
        *acc.entry(*color).or_insert(0usize) += 1;
        acc
    });
    counts.into_iter().collect()
}

/// Longest shortest path from `start`, by breadth-first levels.
fn eccentricity(g: &Graph, start: usize) -> usize {
    let mut seen = vec![false; g.labels.len()];

    seen[start] = true;
    let mut frontier = vec![start];
    let mut depth = 0;
    loop {
        let next: BTreeSet<usize> = frontier
            .iter()
            .flat_map(|v| g.inc[*v].iter().chain(&g.out[*v]))
            .map(|nb| nb.node)
            .filter(|node| !seen[*node])
            .collect();
        if next.is_empty() {
            return depth;
        }
        next.iter().for_each(|node| seen[*node] = true);
        frontier = next.into_iter().collect();
        depth += 1;
    }
}

fn diameter(g: &Graph) -> usize {
    if g.labels.len() > DIAMETER_LIMIT {
        return MAX_LEVELS;
    }
    (0..g.labels.len())
        .map(|v| eccentricity(g, v))
        .max()
        .unwrap_or(0)
}

/// Refined colors and histograms of one graph.
#[derive(Debug, Clone)]
pub struct Wl {
    pub graph: Graph,
    /// `rounds[k]` are the colors after `k` refinements (`rounds[0]` are the labels).
    pub rounds: Vec<Vec<u64>>,
    hists: Vec<Vec<(u64, usize)>>,
    pub calls: usize,
    /// Only parameter/return glue and calls with recorded identities; not semantic purity.
    pub(crate) forwarding_shape: bool,
    /// Ordinary closure bodies are absent even when every recorded node aligns.
    pub(crate) opaque_closure: bool,
    pub diameter: usize,
}

impl Wl {
    pub fn new(pdg: &Pdg) -> Wl {
        let calls = pdg.nodes.iter().fold(
            CallSummary {
                calls: 0,
                forwarding_shape: true,
                opaque_closure: false,
            },
            CallSummary::observe,
        );

        let g = graph(pdg);
        let mut rounds = vec![g.labels.clone()];
        for _ in 1..MAX_LEVELS {
            rounds.push(refine(&g, rounds.last().expect("non-empty")));
        }
        Wl {
            hists: rounds.iter().map(|colors| histogram(colors)).collect(),
            calls: calls.calls,
            forwarding_shape: calls.forwarding_shape,
            opaque_closure: calls.opaque_closure,
            diameter: diameter(&g),
            rounds,
            graph: g,
        }
    }

    /// Color histogram at refinement `level` (0 = raw labels).
    pub fn histogram_at(&self, level: usize) -> &[(u64, usize)] {
        &self.hists[level]
    }

    pub fn nodes(&self) -> usize {
        self.graph.labels.len()
    }

    /// Colors used when comparing against `other`: refinement depth is capped by the
    /// smaller graph's diameter.
    pub fn colors_with(&self, other: &Wl) -> &[u64] {
        &self.rounds[levels(self, other) - 1]
    }
}

/// Stable fingerprint of a graph's structure: its deepest color histogram, hashed. Node
/// order, lines and names do not matter (names are not in the labels); a change to the
/// dependence structure does.
pub fn shape_hash(pdg: &Pdg) -> u64 {
    let wl = Wl::new(pdg);
    let deepest = wl.hists.last().map_or(&[][..], Vec::as_slice);

    deepest.iter().fold(FNV_OFFSET, |acc, (color, count)| {
        combine(combine(acc, *color), *count as u64)
    })
}

pub fn levels(a: &Wl, b: &Wl) -> usize {
    (a.diameter.min(b.diameter) + 1).clamp(1, MAX_LEVELS)
}

/// Per-level weights `(h - i + 1)`, normalized by `h` in the formula: earlier levels count more.
pub fn weights(levels: usize) -> Vec<u64> {
    (1..=levels).map(|i| (levels - i + 1) as u64).collect()
}

pub fn intersection(a: &[(u64, usize)], b: &[(u64, usize)]) -> usize {
    let (mut i, mut j, mut total) = (0, 0, 0);
    while i < a.len() && j < b.len() {
        match a[i].0.cmp(&b[j].0) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                total += a[i].1.min(b[j].1);
                i += 1;
                j += 1;
            }
        }
    }
    total
}

/// `sum_i w_i |hist_i(A) ∩ hist_i(B)| / sum_i w_i max(|A|, |B|)`, in thousandths.
pub fn similarity_milli(a: &Wl, b: &Wl) -> u32 {
    let h = levels(a, b);
    let denom_nodes = a.nodes().max(b.nodes()) as u64;
    let weights = weights(h);
    let num: u64 = (0..h)
        .map(|i| weights[i] * intersection(&a.hists[i], &b.hists[i]) as u64)
        .sum();
    let den: u64 = weights.iter().map(|w| w * denom_nodes).sum();

    (num * 1000).checked_div(den).unwrap_or(0) as u32
}

/// True when a graph is big enough to be worth mining (size floor).
pub fn mineable(w: &Wl) -> bool {
    w.nodes() >= MIN_NODES && w.calls >= MIN_CALLS
}

/// Scale-ratio pre-filter, applied before any kernel work.
pub fn comparable(a: &Wl, b: &Wl) -> bool {
    let (small, large) = (a.nodes().min(b.nodes()), a.nodes().max(b.nodes()));
    mineable(a) && mineable(b) && large <= small * MAX_SCALE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{Pdg, Receiver, SigFacts, TyShape};
    use crate::pdgtest::*;

    fn sim(a: &Pdg, b: &Pdg) -> u32 {
        similarity_milli(&Wl::new(a), &Wl::new(b))
    }

    fn sorted_hist(pdg: &Pdg, level: usize) -> Vec<(u64, usize)> {
        Wl::new(pdg).hists[level].clone()
    }

    #[test]
    fn hash_stable_golden() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(combine(1, 2), combine(1, 2));
        assert_ne!(combine(1, 2), combine(2, 1));
        let dogs = Wl::new(&emit_loop("Dog", "bark"));
        let cats = Wl::new(&emit_loop("Cat", "meow"));
        // Names and workspace types are not part of the labels, so these are the same graph.
        assert_eq!(dogs.rounds, cats.rounds);
        assert_eq!(
            label(&getter_call("Dog", "bark")),
            "call:fn(&Self) -> String"
        );
    }

    #[test]
    fn shape_hash_ignores_names_and_node_order() {
        let dogs = emit_loop("Dog", "bark");
        let cats = emit_loop("Cat", "meow");

        assert_eq!(shape_hash(&dogs), shape_hash(&cats));
        for shift in 1..dogs.nodes.len() {
            assert_eq!(shape_hash(&dogs), shape_hash(&permuted(&dogs, shift)));
        }
    }

    #[test]
    fn shape_hash_changes_with_structure() {
        let base = emit_loop("Dog", "bark");
        let mut extra = base.clone();
        extra.nodes.push(getter_call("Dog", "bark"));
        extra.edges.push(data(2, 6, 0));

        assert_ne!(shape_hash(&base), shape_hash(&extra));
    }

    #[test]
    fn shape_hash_is_pinned() {
        // Golden: a change here invalidates every committed rstyle-ignore.toml.
        assert_eq!(
            format!("{:016x}", shape_hash(&emit_loop("Dog", "bark"))),
            "f440c2186883aa43"
        );
    }

    #[test]
    fn renaming_locals_invariant() {
        let base = emit_loop("Dog", "bark");
        for shift in 1..base.nodes.len() {
            let moved = permuted(&base, shift);
            assert_eq!(sim(&base, &moved), 1000, "shift {shift}");
            assert_eq!(sorted_hist(&base, 3), sorted_hist(&moved, 3));
        }
    }

    #[test]
    fn reordering_independent_stmts_invariant() {
        let mut one = emit_loop("Dog", "bark");
        let mut two = one.clone();
        // Two unrelated calls added in opposite order.
        for pdg in [&mut one, &mut two] {
            pdg.nodes.push(getter_call("Dog", "bark"));
            pdg.nodes.push(push_call());
        }
        two.nodes.swap(6, 7);
        assert_eq!(sim(&one, &two), 1000);
    }

    #[test]
    fn extra_unrelated_call_stays_above_threshold() {
        let base = emit_loop("Dog", "bark");
        let mut audited = base.clone();
        audited.nodes.push(call(
            "k::audit",
            SigFacts {
                ret: TyShape::Tuple(vec![]),
                ..crate::shape::fixtures::method("k::X", Receiver::None, vec![], TyShape::Other)
            },
        ));
        let value = sim(&base, &audited);
        assert!(value >= 800, "{value}");
        assert!(value < 1000);
    }

    #[test]
    fn callee_sig_change_changes_labels() {
        let base = emit_loop("Dog", "bark");
        let mut changed = base.clone();
        changed.nodes[4].callee_sig.as_mut().expect("sig").receiver = Receiver::RefMut;
        assert_ne!(label(&base.nodes[4]), label(&changed.nodes[4]));
        assert!(sim(&base, &changed) < 1000);
        assert_eq!(
            label(&emit_loop("Cat", "meow").nodes[4]),
            label(&base.nodes[4])
        );
    }

    #[test]
    fn iteration_weights_decrease() {
        let w = weights(4);
        assert_eq!(w, [4, 3, 2, 1]);
        assert!(w.windows(2).all(|pair| pair[0] > pair[1]));
        assert_eq!(weights(1), [1]);
    }

    #[test]
    fn size_floor_and_scale_ratio_filter() {
        let small = Wl::new(&emit_loop("Dog", "bark"));
        let tiny = Wl::new(&Pdg {
            nodes: vec![push_call()],
            edges: vec![],
        });
        assert!(mineable(&small));
        assert!(!mineable(&tiny));
        assert!(!comparable(&small, &tiny));
        let mut wide = emit_loop("Dog", "bark");
        wide.nodes.extend((0..7).map(|_| push_call()));
        assert!(!comparable(&small, &Wl::new(&wide)));
        assert!(comparable(&small, &Wl::new(&emit_loop("Cat", "meow"))));
    }
}
