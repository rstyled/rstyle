//! Serializable trace of one `wl::similarity_milli` computation.
//! Colors are interned to small ints across both graphs, so equal int means equal hash.

use crate::align::{self, Ctx, Hole, Stage};
use crate::facts::{EdgeKind, FnFacts, Pdg};
use crate::wl::{self, Wl};
use serde::Serialize;
use std::collections::BTreeMap;

pub const TRACE_VERSION: u32 = 2;

/// One function as the trace sees it.
pub struct Side<'a> {
    pub id: String,
    pub path: String,
    pub line: usize,
    pub source: String,
    pub pdg: &'a Pdg,
}

impl<'a> Side<'a> {
    pub fn from_facts(f: &'a FnFacts) -> Option<Side<'a>> {
        Some(Side {
            id: f.id.clone(),
            path: f.path.clone(),
            line: f.line,
            source: f.source.clone(),
            pdg: f.pdg.as_ref()?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NodeView {
    pub idx: usize,
    pub kind: String,
    pub label: String,
    pub callee: Option<String>,
    pub detail: String,
    pub line: usize,
}

#[derive(Debug, Serialize)]
pub struct EdgeView {
    pub from: usize,
    pub to: usize,
    pub kind: String,
    pub arg_pos: usize,
}

#[derive(Debug, Serialize)]
pub struct GraphView {
    pub fn_id: String,
    pub path: String,
    pub first_line: usize,
    pub source: String,
    pub nodes: Vec<NodeView>,
    pub edges: Vec<EdgeView>,
    pub diameter: usize,
}

#[derive(Debug, Serialize)]
pub struct Term {
    pub node: usize,
    pub tag: String,
    pub prev: u32,
}

#[derive(Debug, Serialize)]
pub struct Recipe {
    pub node: usize,
    pub own_prev: u32,
    #[serde(rename = "in")]
    pub inward: Vec<Term>,
    pub out: Vec<Term>,
}

#[derive(Debug, Serialize)]
pub struct Round {
    pub level: usize,
    pub graph: usize,
    pub colors: Vec<u32>,
    /// Empty at level 0 (colors are raw labels).
    pub recipes: Vec<Recipe>,
}

#[derive(Debug, Serialize)]
pub struct HistView {
    pub level: usize,
    /// False for levels beyond `levels_used`; shown but not scored.
    pub used: bool,
    pub weight: u64,
    pub a: BTreeMap<u32, usize>,
    pub b: BTreeMap<u32, usize>,
    pub intersection: usize,
    /// Display-only greedy pairing (index order) of nodes with equal color.
    pub matched: Vec<(usize, usize)>,
}

#[derive(Debug, Serialize)]
pub struct Comparable {
    pub ok: bool,
    pub nodes: [usize; 2],
    pub calls: [usize; 2],
    pub min_nodes: usize,
    pub min_calls: usize,
    pub scale_limit: usize,
}

#[derive(Debug, Serialize)]
pub struct AlignView {
    pub coverage_milli: u32,
    pub pairs: Vec<(usize, usize)>,
    pub holes: Vec<Hole>,
    /// Matching after each pipeline step, in order; the last equals `pairs`.
    pub stages: Vec<Stage>,
}

#[derive(Debug, Serialize)]
pub struct Trace {
    pub version: u32,
    pub threshold_milli: u32,
    pub graphs: Vec<GraphView>,
    pub levels_used: usize,
    pub max_levels: usize,
    pub weights: Vec<u64>,
    pub denominator: u64,
    pub comparable: Comparable,
    pub rounds: Vec<Round>,
    pub histograms: Vec<HistView>,
    pub numerator: u64,
    pub sim_milli: u32,
    pub align: AlignView,
}

#[derive(Default)]
struct Interner(BTreeMap<u64, u32>);

impl Interner {
    fn id(&mut self, hash: u64) -> u32 {
        let next = self.0.len() as u32;
        *self.0.entry(hash).or_insert(next)
    }
}

fn kind_name(kind: EdgeKind) -> &'static str {
    match kind {
        EdgeKind::Data => "data",
        EdgeKind::Ctrl => "ctrl",
    }
}

fn graph_view(side: &Side<'_>, w: &Wl) -> GraphView {
    let nodes = side.pdg.nodes.iter().enumerate();
    GraphView {
        fn_id: side.id.clone(),
        path: side.path.clone(),
        first_line: side.line,
        source: side.source.clone(),
        nodes: nodes
            .map(|(idx, n)| NodeView {
                idx,
                kind: format!("{:?}", n.kind).to_lowercase(),
                label: wl::label(n),
                callee: n.callee_id.clone(),
                detail: n.detail.clone(),
                line: n.line,
            })
            .collect(),
        edges: side
            .pdg
            .edges
            .iter()
            .map(|e| EdgeView {
                from: e.from,
                to: e.to,
                kind: kind_name(e.kind).into(),
                arg_pos: e.arg_pos,
            })
            .collect(),
        diameter: w.diameter,
    }
}

fn recipe(v: usize, pdg: &Pdg, prev: &[u32]) -> Recipe {
    let n = pdg.nodes.len();
    let terms = |inward: bool| {
        let mut out: Vec<Term> = pdg
            .edges
            .iter()
            .filter(|e| e.from < n && e.to < n)
            .filter(|e| (if inward { e.to } else { e.from }) == v)
            .map(|e| Term {
                node: if inward { e.from } else { e.to },
                tag: format!("{}@{}", kind_name(e.kind), e.arg_pos),
                prev: prev[if inward { e.from } else { e.to }],
            })
            .collect();
        out.sort_by(|x, y| (&x.tag, x.prev, x.node).cmp(&(&y.tag, y.prev, y.node)));
        out
    };
    Recipe {
        node: v,
        own_prev: prev[v],
        inward: terms(true),
        out: terms(false),
    }
}

/// Interned colors of every level and graph, in (level, A, B) order.
fn interned(ws: [&Wl; 2], levels: usize) -> Vec<[Vec<u32>; 2]> {
    let mut interner = Interner::default();
    (0..levels)
        .map(|k| ws.map(|w| w.rounds[k].iter().map(|h| interner.id(*h)).collect()))
        .collect()
}

fn rounds(sides: [&Side<'_>; 2], colors: &[[Vec<u32>; 2]]) -> Vec<Round> {
    let mut out = Vec::new();
    for (level, pair) in colors.iter().enumerate() {
        for (graph, cs) in pair.iter().enumerate() {
            let recipes = match level {
                0 => vec![],
                _ => (0..cs.len())
                    .map(|v| recipe(v, sides[graph].pdg, &colors[level - 1][graph]))
                    .collect(),
            };
            out.push(Round {
                level,
                graph,
                colors: cs.clone(),
                recipes,
            });
        }
    }
    out
}

fn counts(colors: &[u32]) -> BTreeMap<u32, usize> {
    colors.iter().fold(BTreeMap::new(), |mut m, c| {
        *m.entry(*c).or_default() += 1;
        m
    })
}

fn greedy_pairs(a: &[u32], b: &[u32]) -> Vec<(usize, usize)> {
    let mut used = vec![false; b.len()];
    let mut pairs = Vec::new();
    for (i, c) in a.iter().enumerate() {
        let free = (0..b.len()).find(|j| !used[*j] && b[*j] == *c);
        if let Some(j) = free {
            used[j] = true;
            pairs.push((i, j));
        }
    }
    pairs
}

fn hist_view(level: usize, pair: &[Vec<u32>; 2], used: Option<u64>) -> HistView {
    let (a, b) = (counts(&pair[0]), counts(&pair[1]));
    let intersection = a
        .iter()
        .map(|(c, n)| (*n).min(b.get(c).copied().unwrap_or(0)))
        .sum();
    HistView {
        level,
        used: used.is_some(),
        weight: used.unwrap_or(0),
        a,
        b,
        intersection,
        matched: greedy_pairs(&pair[0], &pair[1]),
    }
}

fn comparable(ws: [&Wl; 2]) -> Comparable {
    Comparable {
        ok: wl::comparable(ws[0], ws[1]),
        nodes: ws.map(|w| w.nodes()),
        calls: ws.map(|w| w.calls),
        min_nodes: wl::MIN_NODES,
        min_calls: wl::MIN_CALLS,
        scale_limit: wl::MAX_SCALE,
    }
}

pub fn trace(a: &Side<'_>, b: &Side<'_>, threshold_milli: u32) -> Trace {
    let (wa, wb) = (Wl::new(a.pdg), Wl::new(b.pdg));
    let h = wl::levels(&wa, &wb);
    let weights = wl::weights(h);
    let colors = interned([&wa, &wb], wl::MAX_LEVELS);
    let histograms: Vec<HistView> = colors
        .iter()
        .enumerate()
        .map(|(k, pair)| hist_view(k, pair, weights.get(k).copied()))
        .collect();
    let numerator: u64 = histograms
        .iter()
        .map(|x| x.weight * x.intersection as u64)
        .sum();
    let denominator = weights.iter().sum::<u64>() * wa.nodes().max(wb.nodes()) as u64;
    let (al, stages) = align::align_staged(&Ctx::new(a.pdg, &wa, b.pdg, &wb));

    Trace {
        version: TRACE_VERSION,
        threshold_milli,
        graphs: vec![graph_view(a, &wa), graph_view(b, &wb)],
        levels_used: h,
        max_levels: wl::MAX_LEVELS,
        weights,
        denominator,
        comparable: comparable([&wa, &wb]),
        rounds: rounds([a, b], &colors),
        histograms,
        numerator,
        sim_milli: (numerator * 1000).checked_div(denominator).unwrap_or(0) as u32,
        align: AlignView {
            coverage_milli: al.coverage_milli,
            pairs: al.pairs,
            holes: al.holes,
            stages,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdgtest::*;

    fn side(pdg: &Pdg) -> Side<'_> {
        Side {
            id: "t::f".into(),
            path: "src/lib.rs".into(),
            line: 1,
            source: "fn f() {}".into(),
            pdg,
        }
    }

    #[test]
    fn trace_score_equals_similarity_milli() {
        let (a, b) = (emit_loop("Dog", "bark"), emit_loop("Cat", "meow"));
        let t = trace(&side(&a), &side(&b), 600);
        assert_eq!(
            t.sim_milli,
            wl::similarity_milli(&Wl::new(&a), &Wl::new(&b))
        );
    }

    #[test]
    fn identical_graphs_score_1000_with_full_levels() {
        let g = emit_loop("Dog", "bark");
        let t = trace(&side(&g), &side(&g), 600);
        assert_eq!(t.sim_milli, 1000);
        let n = g.nodes.len();
        assert!(
            t.histograms
                .iter()
                .filter(|h| h.used)
                .all(|h| h.intersection == n)
        );
    }

    #[test]
    fn colors_are_interned_across_both_graphs() {
        let g = emit_loop("Dog", "bark");
        let t = trace(&side(&g), &side(&g), 600);
        let level0: Vec<_> = t.rounds.iter().filter(|r| r.level == 0).collect();
        assert_eq!(level0[0].colors, level0[1].colors);
    }

    #[test]
    fn recipes_list_neighbours_with_edge_tags() {
        let g = emit_loop("Dog", "bark");
        let t = trace(&side(&g), &side(&g), 600);
        let r = t
            .rounds
            .iter()
            .find(|r| r.level == 1 && r.graph == 0)
            .unwrap();
        let edges = g.edges.len();
        let total: usize = r.recipes.iter().map(|x| x.inward.len()).sum();
        assert_eq!(total, edges);
        assert!(
            r.recipes
                .iter()
                .flat_map(|x| &x.out)
                .all(|t| t.tag.contains('@'))
        );
    }

    #[test]
    fn last_alignment_stage_equals_the_final_pairs() {
        let (a, b) = (emit_loop("Dog", "bark"), emit_loop("Cat", "meow"));
        let t = trace(&side(&a), &side(&b), 600);
        let names: Vec<_> = t.align.stages.iter().map(|s| s.name).collect();
        assert_eq!(names.first(), Some(&"seed"));
        assert_eq!(names.last(), Some(&"final"));
        assert_eq!(t.align.stages.last().unwrap().pairs, t.align.pairs);
        assert_eq!(t.align.holes.len(), 2);
    }

    #[test]
    fn trace_is_deterministic() {
        let (a, b) = (emit_loop("Dog", "bark"), emit_loop("Cat", "meow"));
        let j = |t: &Trace| serde_json::to_string(t).unwrap();
        assert_eq!(
            j(&trace(&side(&a), &side(&b), 600)),
            j(&trace(&side(&a), &side(&b), 600))
        );
    }
}
