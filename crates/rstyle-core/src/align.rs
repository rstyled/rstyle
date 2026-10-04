//! Alignment of two dependence graphs: which node corresponds to which, and where the
//! matched nodes differ (the *holes*). Holes are the substitution table of the
//! anti-unification: `T0 = Dog | Cat`, `M0 = bark | meow`.

use crate::facts::{NodeKind, Pdg, PdgNode};
use crate::shape::adt_diffs;
use crate::wl::{Nb, Wl};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HoleKind {
    Type,
    /// Callee with a receiver.
    Method,
    /// Callee without a receiver, or a fn item passed as a value.
    FreeFn,
    Literal,
    Field,
    /// Operator of a canonical comparison (`<` vs `==` against a literal bound).
    Op,
}

impl HoleKind {
    fn prefix(self) -> char {
        match self {
            Self::Type => 'T',
            Self::Method => 'M',
            Self::FreeFn => 'F',
            Self::Literal => 'L',
            Self::Field => 'D',
            Self::Op => 'O',
        }
    }
}

impl Hole {
    /// Variable name of the `index`-th hole of a kind: `T0`, `M1`, ...
    pub fn var_name(kind: HoleKind, index: usize) -> String {
        format!("{}{index}", kind.prefix())
    }
}

/// One substitution `a -> b`. Every site where the same pair occurs shares the variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hole {
    pub kind: HoleKind,
    pub var: String,
    pub a: String,
    pub b: String,
    /// Aligned node pairs `(node in a, node in b)` where the substitution applies.
    pub sites: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alignment {
    pub pairs: Vec<(usize, usize)>,
    pub holes: Vec<Hole>,
    /// Aligned pairs over the larger node count, in thousandths.
    pub coverage_milli: u32,
}

/// Everything alignment reads, bundled so helpers stay within the parameter limit.
pub struct Ctx<'a> {
    pub pa: &'a Pdg,
    pub pb: &'a Pdg,
    pub wa: &'a Wl,
    pub wb: &'a Wl,
    ca: &'a [u64],
    cb: &'a [u64],
    oa: Vec<usize>,
    ob: Vec<usize>,
}

impl<'a> Ctx<'a> {
    pub fn new(pa: &'a Pdg, wa: &'a Wl, pb: &'a Pdg, wb: &'a Wl) -> Ctx<'a> {
        let (ca, cb) = (wa.colors_with(wb), wb.colors_with(wa));
        Ctx {
            pa,
            pb,
            wa,
            wb,
            ca,
            cb,
            oa: ordinals(wa, ca),
            ob: ordinals(wb, cb),
        }
    }
}

impl<'a> Ctx<'a> {
    /// Number of WL levels both graphs support (`wl::levels`).
    fn levels_used(&self) -> usize {
        crate::wl::levels(self.wa, self.wb)
    }

    /// The same graphs compared by the colors of WL refinement `level` (0 = raw labels).
    fn at_level(&self, level: usize) -> Ctx<'a> {
        Ctx {
            ca: &self.wa.rounds[level],
            cb: &self.wb.rounds[level],
            oa: self.oa.clone(),
            ob: self.ob.clone(),
            ..*self
        }
    }
}

/// Depth-first visit order. Children are taken by (edge tag, color, index).
struct Dfs<'a> {
    graph: &'a crate::wl::Graph,
    colors: &'a [u64],
    seen: Vec<bool>,
    order: Vec<usize>,
}

impl Dfs<'_> {
    fn children(&self, v: usize) -> Vec<usize> {
        let mut kids: Vec<&Nb> = self.graph.out[v].iter().collect();
        kids.sort_by_key(|nb| (nb.tag, self.colors[nb.node], nb.node));
        kids.iter().rev().map(|nb| nb.node).collect()
    }

    fn from(&mut self, start: usize) {
        let mut stack = vec![start];
        while let Some(v) = stack.pop() {
            if !std::mem::replace(&mut self.seen[v], true) {
                self.order.push(v);
                stack.extend(self.children(v));
            }
        }
    }
}

/// Deterministic DFS ordinal of every node: roots by (color, index), then anything left
/// (cycles) by index. Symmetric graphs still get one fixed order.
fn ordinals(w: &Wl, colors: &[u64]) -> Vec<usize> {
    let n = colors.len();
    let mut roots: Vec<usize> = (0..n).filter(|v| w.graph.inc[*v].is_empty()).collect();
    roots.sort_by_key(|v| (colors[*v], *v));
    let mut dfs = Dfs {
        graph: &w.graph,
        colors,
        seen: vec![false; n],
        order: Vec::with_capacity(n),
    };
    roots
        .into_iter()
        .chain(0..n)
        .for_each(|start| dfs.from(start));
    let mut ordinal = vec![0; n];
    dfs.order
        .iter()
        .enumerate()
        .for_each(|(i, v)| ordinal[*v] = i);
    ordinal
}

#[derive(Debug, Clone)]
pub struct Matching {
    pub fwd: Vec<Option<usize>>,
    back: Vec<Option<usize>>,
}

impl Matching {
    pub fn new(na: usize, nb: usize) -> Matching {
        Matching {
            fwd: vec![None; na],
            back: vec![None; nb],
        }
    }

    pub fn link(mut self, a: usize, b: usize) -> Matching {
        self.fwd[a] = Some(b);
        self.back[b] = Some(a);
        self
    }

    fn unlink(mut self, a: usize) -> Matching {
        if let Some(b) = self.fwd[a].take() {
            self.back[b] = None;
        }
        self
    }

    pub fn pairs(&self) -> Vec<(usize, usize)> {
        let linked = self.fwd.iter().enumerate();
        linked.filter_map(|(a, b)| b.map(|b| (a, b))).collect()
    }
}

fn has_edge(list: &[Nb], node: usize, tag: u64) -> bool {
    list.iter().any(|nb| nb.node == node && nb.tag == tag)
}

/// Neighbours of `a` that are matched, split into those the counterpart `b` mirrors
/// (agree) and those it does not (conflict).
fn evidence(ctx: &Ctx<'_>, m: &Matching, a: usize, b: usize) -> (usize, usize) {
    let sides = [
        (&ctx.wa.graph.inc[a], &ctx.wb.graph.inc[b]),
        (&ctx.wa.graph.out[a], &ctx.wb.graph.out[b]),
    ];
    sides
        .iter()
        .flat_map(|(mine, theirs)| {
            mine.iter().filter_map(|nb| {
                let image = m.fwd[nb.node]?;
                Some(has_edge(theirs, image, nb.tag))
            })
        })
        .fold((0, 0), |(agree, conflict), ok| {
            (agree + usize::from(ok), conflict + usize::from(!ok))
        })
}

/// Edges of `b` into matched nodes that `a` does not mirror.
fn reverse_conflicts(ctx: &Ctx<'_>, m: &Matching, a: usize, b: usize) -> usize {
    let sides = [
        (&ctx.wb.graph.inc[b], &ctx.wa.graph.inc[a]),
        (&ctx.wb.graph.out[b], &ctx.wa.graph.out[a]),
    ];
    sides
        .iter()
        .flat_map(|(theirs, mine)| {
            theirs.iter().filter_map(|nb| {
                let preimage = m.back[nb.node]?;
                Some(!has_edge(mine, preimage, nb.tag))
            })
        })
        .filter(|missing| *missing)
        .count()
}

fn classes(colors: &[u64], ordinal: &[usize]) -> BTreeMap<u64, Vec<usize>> {
    let mut by_color: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (v, color) in colors.iter().enumerate() {
        by_color.entry(*color).or_default().push(v);
    }
    by_color
        .values_mut()
        .for_each(|nodes| nodes.sort_by_key(|v| ordinal[*v]));
    by_color
}

fn unique_pairs(ctx: &Ctx<'_>) -> Vec<(usize, usize)> {
    let (ka, kb) = (classes(ctx.ca, &ctx.oa), classes(ctx.cb, &ctx.ob));
    ka.iter()
        .filter_map(
            |(color, nodes)| match (nodes.as_slice(), kb.get(color)?.as_slice()) {
                ([a], [b]) => Some((*a, *b)),
                _ => None,
            },
        )
        .collect()
}

/// Best unmatched same-color partner of `a`: most mirrored neighbours, then lowest ordinal.
fn best_partner(
    ctx: &Ctx<'_>,
    m: &Matching,
    a: usize,
    kb: &BTreeMap<u64, Vec<usize>>,
) -> Option<(usize, usize)> {
    let candidates = kb.get(&ctx.ca[a])?.iter().filter(|b| m.back[**b].is_none());
    let scored = candidates.map(|b| (evidence(ctx, m, a, *b), *b));
    let best = scored
        .filter(|((_, conflict), _)| *conflict == 0)
        .min_by_key(|((agree, _), b)| (usize::MAX - agree, ctx.ob[*b]))?;
    Some((best.0.0, best.1))
}

/// One pass over unmatched nodes in ordinal order. With `need_evidence`, a node is only
/// matched when a neighbour already vouches for the partner.
fn pass(ctx: &Ctx<'_>, m: Matching, need_evidence: bool) -> (Matching, bool) {
    let kb = classes(ctx.cb, &ctx.ob);
    let mut order: Vec<usize> = (0..ctx.ca.len()).collect();
    order.sort_by_key(|a| ctx.oa[*a]);
    order.into_iter().fold((m, false), |(m, moved), a| {
        let pick = if m.fwd[a].is_some() {
            None
        } else {
            best_partner(ctx, &m, a, &kb)
        };
        match pick {
            Some((agree, b)) if agree > 0 || !need_evidence => (m.link(a, b), true),
            _ => (m, moved),
        }
    })
}

/// Repeats evidence-backed passes to a fixpoint; `greedy` then matches what is left by ordinal.
fn settle(ctx: &Ctx<'_>, start: Matching, greedy: bool) -> Matching {
    let mut m = start;
    loop {
        let (next, moved) = pass(ctx, m, true);
        m = next;
        if !moved {
            return if greedy { pass(ctx, m, false).0 } else { m };
        }
    }
}

fn holey(kind: NodeKind) -> bool {
    matches!(kind, NodeKind::Call | NodeKind::Lit | NodeKind::Field)
}

/// Pairs unmatched same-kind leaf-like nodes whose label differs (a different callee
/// signature class, say) but whose matched neighbours all agree.
fn complete(ctx: &Ctx<'_>, start: Matching, strict: bool) -> Matching {
    let mut order: Vec<usize> = (0..ctx.ca.len()).collect();
    order.sort_by_key(|a| ctx.oa[*a]);
    order.into_iter().fold(start, |m, a| {
        if m.fwd[a].is_some() || !holey(ctx.pa.nodes[a].kind) {
            return m;
        }
        let fits = (0..ctx.cb.len())
            .filter(|b| m.back[*b].is_none() && ctx.pb.nodes[*b].kind == ctx.pa.nodes[a].kind);
        let best = fits
            .map(|b| (evidence(ctx, &m, a, b), b))
            .filter(|((agree, conflict), _)| *agree > 0 && *conflict == 0)
            .filter(|(_, b)| !strict || reverse_conflicts(ctx, &m, a, *b) == 0)
            .min_by_key(|((agree, _), b)| (usize::MAX - agree, ctx.ob[*b]));
        match best {
            Some((_, b)) => m.link(a, b),
            None => m,
        }
    })
}

fn violations(ctx: &Ctx<'_>, m: &Matching, a: usize, b: usize) -> usize {
    evidence(ctx, m, a, b).1 + reverse_conflicts(ctx, m, a, b)
}

/// Drops pairs whose edges are not mirrored, worst first, until the matching is
/// edge-consistent.
pub fn prune(ctx: &Ctx<'_>, start: Matching) -> Matching {
    let mut m = start;
    loop {
        let worst = m
            .pairs()
            .into_iter()
            .map(|(a, b)| (violations(ctx, &m, a, b), a))
            .filter(|(count, _)| *count > 0)
            .max();
        match worst {
            Some((_, a)) => m = m.unlink(a),
            None => return m,
        }
    }
}

fn is_method(node: &PdgNode) -> bool {
    node.callee_sig
        .as_ref()
        .is_some_and(|sig| sig.receiver != crate::facts::Receiver::None)
}

fn callee_diff(a: &PdgNode, b: &PdgNode) -> Option<(HoleKind, String, String)> {
    let (x, y) = (a.callee_id.as_ref()?, b.callee_id.as_ref()?);
    let kind = if a.kind == NodeKind::Call && (is_method(a) || is_method(b)) {
        HoleKind::Method
    } else {
        HoleKind::FreeFn
    };
    (x != y).then(|| (kind, x.clone(), y.clone()))
}

fn text_diff(a: &PdgNode, b: &PdgNode) -> Option<(HoleKind, String, String)> {
    let kind = match a.kind {
        NodeKind::Lit if a.callee_id.is_none() => HoleKind::Literal,
        NodeKind::Field => HoleKind::Field,
        NodeKind::Op if a.detail.starts_with("cmp:") => {
            let (x, y) = (&a.detail[4..], &b.detail[4..]);
            return (a.detail != b.detail && b.detail.starts_with("cmp:"))
                .then(|| (HoleKind::Op, x.to_string(), y.to_string()));
        }
        _ => return None,
    };
    (a.detail != b.detail).then(|| (kind, a.detail.clone(), b.detail.clone()))
}

fn type_diffs(a: &PdgNode, b: &PdgNode) -> Vec<(HoleKind, String, String)> {
    match (&a.ty, &b.ty) {
        (Some(x), Some(y)) => adt_diffs(x, y)
            .into_iter()
            .map(|(p, q)| (HoleKind::Type, p, q))
            .collect(),
        _ => vec![],
    }
}

fn node_diffs(a: &PdgNode, b: &PdgNode) -> Vec<(HoleKind, String, String)> {
    callee_diff(a, b)
        .into_iter()
        .chain(text_diff(a, b))
        .chain(type_diffs(a, b))
        .collect()
}

type HoleKey = (HoleKind, String, String);

fn hole_table(ctx: &Ctx<'_>, pairs: &[(usize, usize)]) -> BTreeMap<HoleKey, Vec<(usize, usize)>> {
    let diffs = pairs.iter().flat_map(|&(a, b)| {
        let found = node_diffs(&ctx.pa.nodes[a], &ctx.pb.nodes[b]);
        found.into_iter().map(move |key| (key, (a, b)))
    });
    diffs.fold(BTreeMap::new(), |mut table, (key, site)| {
        table.entry(key).or_default().push(site);
        table
    })
}

/// Substitution table of an alignment: equal `(kind, a, b)` triples share one variable.
pub fn holes_of(ctx: &Ctx<'_>, pairs: &[(usize, usize)]) -> Vec<Hole> {
    let mut ordered: Vec<_> = hole_table(ctx, pairs).into_iter().collect();
    ordered.sort_by_key(|(key, sites)| (sites[0].0, key.0));
    let mut counts: BTreeMap<HoleKind, usize> = BTreeMap::new();
    let mut name = |kind: HoleKind| {
        let count = counts.entry(kind).or_insert(0);
        *count += 1;
        Hole::var_name(kind, *count - 1)
    };
    ordered
        .into_iter()
        .map(|((kind, a, b), sites)| Hole {
            kind,
            var: name(kind),
            a,
            b,
            sites,
        })
        .collect()
}

/// Links the pairs that are unique in their color class and still unmatched.
fn seeded(ctx: &Ctx<'_>, m: Matching) -> Matching {
    unique_pairs(ctx).into_iter().fold(m, |m, (a, b)| {
        if m.fwd[a].is_none() && m.back[b].is_none() {
            m.link(a, b)
        } else {
            m
        }
    })
}

/// Pairs after one pipeline step (cumulative, so `prune` steps can shrink the set).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Stage {
    pub name: &'static str,
    pub pairs: Vec<(usize, usize)>,
}

pub fn align(ctx: &Ctx<'_>) -> Alignment {
    align_staged(ctx).0
}

/// Pipeline variants, for the ablation (`examples/align_ablation.rs`). `Baseline` is `align`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Baseline,
    /// Skip the refined-color stages; start from raw labels.
    LabelsOnly,
    /// No prune after `complete`.
    NoFinalPrune,
    /// `complete` also rejects pairs with reverse conflicts (what prune checks).
    CompleteChecked,
    /// Seed and settle at every WL level from deepest to raw labels, pruning after each.
    CoarseToFine,
    /// `CoarseToFine` plus `CompleteChecked`.
    Combined,
}

type Step<'a> = (&'static str, Box<dyn Fn(Matching) -> Matching + 'a>);

fn grow<'a>(c: &'a Ctx<'a>, name: &'static str, greedy: bool) -> Step<'a> {
    (name, Box::new(move |m| settle(c, seeded(c, m), greedy)))
}

fn prune_step<'a>(ctx: &'a Ctx<'a>, name: &'static str) -> Step<'a> {
    (name, Box::new(|m| prune(ctx, m)))
}

const LEVEL_NAMES: [&str; 4] = ["level0", "level1", "level2", "level3"];

/// Refined-color stages, then raw labels (the pipeline before `complete`).
fn pairing_steps<'a>(ctx: &'a Ctx<'a>, levels: &'a [Ctx<'a>], v: Variant) -> Vec<Step<'a>> {
    let labels = &levels[0];
    match v {
        Variant::LabelsOnly => vec![
            grow(labels, "labels", false),
            prune_step(ctx, "prune_labels"),
        ],
        Variant::CoarseToFine | Variant::Combined => levels
            .iter()
            .enumerate()
            .rev()
            .flat_map(|(k, c)| {
                [
                    grow(c, LEVEL_NAMES[k.min(3)], k == levels.len() - 1),
                    prune_step(ctx, "prune"),
                ]
            })
            .collect(),
        _ => vec![
            ("seed", Box::new(|m| seeded(ctx, m))),
            ("settle", Box::new(|m| settle(ctx, m, true))),
            prune_step(ctx, "prune"),
            // Nodes whose neighbourhood differs still pair by label.
            grow(labels, "labels", false),
            prune_step(ctx, "prune_labels"),
        ],
    }
}

fn steps<'a>(ctx: &'a Ctx<'a>, levels: &'a [Ctx<'a>], v: Variant) -> Vec<Step<'a>> {
    let strict = matches!(v, Variant::CompleteChecked | Variant::Combined);
    let tail: Vec<Step<'a>> = vec![("complete", Box::new(move |m| complete(ctx, m, strict)))];
    let last = (v != Variant::NoFinalPrune).then(|| prune_step(ctx, "final"));
    pairing_steps(ctx, levels, v)
        .into_iter()
        .chain(tail)
        .chain(last)
        .collect()
}

/// `align`, also reporting the matching after every step (for the walkthrough site).
pub fn align_staged(ctx: &Ctx<'_>) -> (Alignment, Vec<Stage>) {
    align_variant(ctx, Variant::Baseline)
}

pub fn align_variant(ctx: &Ctx<'_>, v: Variant) -> (Alignment, Vec<Stage>) {
    let depth = ctx.wa.rounds.len().min(ctx.levels_used());
    let levels: Vec<Ctx<'_>> = (0..depth.max(1)).map(|k| ctx.at_level(k)).collect();
    let empty = Matching::new(ctx.ca.len(), ctx.cb.len());
    let (_, stages) =
        steps(ctx, &levels, v)
            .iter()
            .fold((empty, Vec::new()), |(m, mut stages), (name, run)| {
                let next = run(m);
                stages.push(Stage {
                    name,
                    pairs: next.pairs(),
                });
                (next, stages)
            });
    let pairs = stages.last().map_or_else(Vec::new, |s| s.pairs.clone());
    let larger = ctx.ca.len().max(ctx.cb.len()).max(1);
    let alignment = Alignment {
        coverage_milli: (pairs.len() * 1000 / larger) as u32,
        holes: holes_of(ctx, &pairs),
        pairs,
    };
    (alignment, stages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::TyShape;
    use crate::pdgtest::*;

    fn run(a: &Pdg, b: &Pdg) -> Alignment {
        let (wa, wb) = (Wl::new(a), Wl::new(b));
        align(&Ctx::new(a, &wa, b, &wb))
    }

    fn kinds(al: &Alignment) -> Vec<(HoleKind, String, String)> {
        al.holes
            .iter()
            .map(|h| (h.kind, h.a.clone(), h.b.clone()))
            .collect()
    }

    #[test]
    fn canonical_comparison_operators_differ_as_an_op_hole() {
        let graph = |operator: &str| Pdg {
            nodes: vec![
                node(NodeKind::Loop),
                PdgNode {
                    lit_kind: Some("int".into()),
                    detail: "3".into(),
                    ..node(NodeKind::Lit)
                },
                PdgNode {
                    detail: format!("cmp:{operator}"),
                    ..node(NodeKind::Op)
                },
                node(NodeKind::Case),
            ],
            edges: vec![data(0, 2, 0), data(1, 2, 1), data(2, 3, 0)],
        };
        let al = run(&graph("<"), &graph("=="));
        assert_eq!(al.coverage_milli, 1000);
        assert_eq!(
            kinds(&al),
            [(HoleKind::Op, "<".to_string(), "==".to_string())]
        );
        assert_eq!(al.holes[0].var, "O0");
        assert!(run(&graph("<"), &graph("<")).holes.is_empty());
    }

    #[test]
    fn dog_cat_yields_exactly_t_and_m_holes() {
        let al = run(&emit_loop("Dog", "bark"), &emit_loop("Cat", "meow"));
        assert_eq!(al.pairs.len(), 6);
        assert_eq!(al.coverage_milli, 1000);
        let expected = [
            (HoleKind::Type, "k::Dog".to_string(), "k::Cat".to_string()),
            (
                HoleKind::Method,
                "k::Dog::bark".to_string(),
                "k::Cat::meow".to_string(),
            ),
        ];
        assert_eq!(kinds(&al), expected);
        assert_eq!(
            (al.holes[0].var.as_str(), al.holes[1].var.as_str()),
            ("T0", "M0")
        );
    }

    #[test]
    fn repeated_pair_reuses_hole_var() {
        let al = run(&emit_loop("Dog", "bark"), &emit_loop("Cat", "meow"));
        let t = &al.holes[0];
        // `&[Dog] -> &[Cat]` on the parameter and `&Dog -> &Cat` on the loop item: one variable.
        assert_eq!(t.sites, [(0, 0), (2, 2)]);
        assert_eq!(
            al.holes.iter().filter(|h| h.kind == HoleKind::Type).count(),
            1
        );
    }

    #[test]
    fn same_callee_not_a_hole() {
        let al = run(&emit_loop("Dog", "bark"), &emit_loop("Dog", "bark"));
        assert!(al.holes.is_empty());
        assert_eq!(al.coverage_milli, 1000);
    }

    #[test]
    fn symmetric_graph_alignment_deterministic() {
        let twin = Pdg {
            nodes: vec![
                crate::pdgtest::node(NodeKind::Param),
                crate::pdgtest::node(NodeKind::Param),
                push_call(),
                push_call(),
            ],
            edges: vec![data(0, 2, 0), data(1, 3, 0)],
        };
        let flipped = permuted(&twin, 1);
        let first = run(&twin, &flipped);
        assert_eq!(first.pairs.len(), 4);
        assert!(first.holes.is_empty());
        assert_eq!(first.pairs, run(&twin, &flipped).pairs);
        let (wa, wb) = (Wl::new(&twin), Wl::new(&flipped));
        let ctx = Ctx::new(&twin, &wa, &flipped, &wb);
        let m = first
            .pairs
            .iter()
            .fold(Matching::new(4, 4), |m, p| m.link(p.0, p.1));
        assert_eq!(
            prune(&ctx, m).pairs().len(),
            4,
            "alignment must be edge-consistent"
        );
    }

    #[test]
    fn edge_inconsistent_match_rejected() {
        let linked = Pdg {
            nodes: vec![push_call(), push_call()],
            edges: vec![data(0, 1, 0)],
        };
        let loose = Pdg {
            nodes: vec![push_call(), push_call()],
            edges: vec![],
        };
        let (wa, wb) = (Wl::new(&linked), Wl::new(&loose));
        let ctx = Ctx::new(&linked, &wa, &loose, &wb);
        let both = Matching::new(2, 2).link(0, 0).link(1, 1);
        let kept = prune(&ctx, both).pairs();
        assert_eq!(kept.len(), 1, "one of the two clashing pairs must go");
        assert_eq!(prune(&ctx, Matching::new(2, 2).link(0, 0)).pairs().len(), 1);
    }

    #[test]
    fn differing_sig_class_still_pairs_as_hole() {
        let mut other = emit_loop("Cat", "meow");
        other.nodes[4].callee_sig.as_mut().expect("sig").ret = TyShape::Prim("u8".into());
        let al = run(&emit_loop("Dog", "bark"), &other);
        assert!(al.holes.iter().any(|h| h.kind == HoleKind::Method));
        assert_eq!(al.pairs.len(), 6);
    }

    #[test]
    fn every_variant_fully_aligns_a_dog_cat_pair() {
        let (a, b) = (emit_loop("Dog", "bark"), emit_loop("Cat", "meow"));
        let (wa, wb) = (Wl::new(&a), Wl::new(&b));
        let ctx = Ctx::new(&a, &wa, &b, &wb);
        for v in [
            Variant::Baseline,
            Variant::LabelsOnly,
            Variant::NoFinalPrune,
            Variant::CompleteChecked,
            Variant::CoarseToFine,
            Variant::Combined,
        ] {
            let al = align_variant(&ctx, v).0;
            assert_eq!(al.coverage_milli, 1000, "{v:?}");
            assert_eq!(al.holes.len(), 2, "{v:?}");
        }
    }
}
