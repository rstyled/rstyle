//! Groups similar functions around a template (medoid) instead of chaining pairs, so
//! dissimilar bodies cannot be linked through a middleman (no transitive union-find).

use crate::align::{Alignment, Ctx, HoleKind, align};
use crate::facts::{NodeKind, Pdg};
use crate::normalize::{Rules, canonicalize};
use crate::wl::{Wl, call_class, comparable, similarity_milli};
use std::collections::{BTreeMap, BTreeSet};

/// Blocks larger than this are skipped: their key is too common to say anything.
const BLOCK_CAP: usize = 1000;
/// How many of a function's rarest call classes are used as blocking keys.
const BLOCK_KEYS: usize = 2;
/// Corpora up to this size compare every pair: blocking on the rarest call classes picks
/// classes unique to each function when the corpus is tiny, so parallel functions never meet.
const ALL_PAIRS_LIMIT: usize = 400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Params {
    pub threshold_milli: u32,
    pub min_coverage_milli: u32,
    /// Experiment: also select pairs whose alignment coverage reaches this (besides WL similarity).
    pub pair_coverage_milli: Option<u32>,
    /// Canonicalization rules applied to every graph before hashing and aligning (default: none).
    pub normalize: Rules,
    /// Experiment: bodies with one call count as substantial unless that call only forwards parameters.
    pub single_call_guard: bool,
    /// More differing parts than this and a `parameterize` helper would take a parameter per line.
    pub max_holes: usize,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            threshold_milli: 600,
            min_coverage_milli: 600,
            pair_coverage_milli: None,
            normalize: Rules::default(),
            single_call_guard: false,
            max_holes: 3,
        }
    }
}

/// One hole variable across all members: `values[k]` belongs to `Cluster::members[k]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub kind: HoleKind,
    pub var: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cluster {
    /// Indices into the input slice; the template comes first, the rest ascending.
    pub members: Vec<usize>,
    pub coverage_milli: Vec<u32>,
    pub columns: Vec<Column>,
}

fn classes_of(pdg: &Pdg) -> BTreeSet<String> {
    let calls = pdg.nodes.iter().filter(|n| n.kind == NodeKind::Call);
    calls.map(call_class).collect()
}

/// The rarest call classes (across the corpus) of every function.
fn block_keys(pdgs: &[&Pdg]) -> Vec<Vec<String>> {
    let sets: Vec<BTreeSet<String>> = pdgs.iter().map(|p| classes_of(p)).collect();
    let mut freq: BTreeMap<&str, usize> = BTreeMap::new();
    for class in sets.iter().flatten() {
        *freq.entry(class.as_str()).or_default() += 1;
    }
    sets.iter()
        .map(|set| {
            let mut ranked: Vec<&String> = set.iter().collect();
            ranked.sort_by_key(|c| (freq[c.as_str()], c.as_str()));
            ranked.into_iter().take(BLOCK_KEYS).cloned().collect()
        })
        .collect()
}

type Adjacency = BTreeMap<usize, BTreeSet<usize>>;

fn candidate_pairs(keys: &[Vec<String>]) -> BTreeSet<(usize, usize)> {
    let mut blocks: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, list) in keys.iter().enumerate() {
        for key in list {
            blocks.entry(key).or_default().push(index);
        }
    }
    blocks
        .values()
        .filter(|members| members.len() <= BLOCK_CAP)
        .flat_map(|members| {
            members
                .iter()
                .enumerate()
                .flat_map(|(i, a)| members[i + 1..].iter().map(move |b| (*a, *b)))
        })
        .collect()
}

/// Every pair for small corpora, pairs that share a blocking key otherwise.
fn pairs_to_compare(pdgs: &[&Pdg]) -> BTreeSet<(usize, usize)> {
    if pdgs.len() > ALL_PAIRS_LIMIT {
        return candidate_pairs(&block_keys(pdgs));
    }
    (0..pdgs.len())
        .flat_map(|a| (a + 1..pdgs.len()).map(move |b| (a, b)))
        .collect()
}

/// A comparable pair whose WL similarity reaches the threshold, or (experiment) whose
/// alignment coverage reaches `pair_coverage_milli`.
fn pair_selected(pdgs: &[&Pdg], wls: &[Wl], (a, b): (usize, usize), params: &Params) -> bool {
    if !comparable(&wls[a], &wls[b]) {
        return false;
    }
    similarity_milli(&wls[a], &wls[b]) >= params.threshold_milli
        || params.pair_coverage_milli.is_some_and(|floor| {
            let ctx = Ctx::new(pdgs[a], &wls[a], pdgs[b], &wls[b]);
            align(&ctx).coverage_milli >= floor
        })
}

/// Functions whose kernel similarity reaches the threshold, after the size filters.
fn similar(
    pdgs: &[&Pdg],
    wls: &[Wl],
    pairs: &BTreeSet<(usize, usize)>,
    params: &Params,
) -> Adjacency {
    let mut adjacency = Adjacency::new();
    for &(a, b) in pairs {
        if pair_selected(pdgs, wls, (a, b), params) {
            adjacency.entry(a).or_default().insert(b);
            adjacency.entry(b).or_default().insert(a);
        }
    }
    adjacency
}

/// Unassigned function with the most unassigned neighbours (lowest index on ties).
fn next_seed(adjacency: &Adjacency, free: &BTreeSet<usize>) -> Option<usize> {
    free.iter()
        .map(|v| {
            let degree = adjacency.get(v).map_or(0, |n| n.intersection(free).count());
            (degree, *v)
        })
        .filter(|(degree, _)| *degree > 0)
        .min_by_key(|(degree, v)| (usize::MAX - degree, *v))
        .map(|(_, v)| v)
}

/// The group member with the highest total similarity to the others.
fn medoid(wls: &[Wl], group: &[usize]) -> usize {
    let total = |v: usize| -> u32 {
        group
            .iter()
            .filter(|w| **w != v)
            .map(|w| similarity_milli(&wls[v], &wls[*w]))
            .sum()
    };
    group
        .iter()
        .copied()
        .min_by_key(|v| (u32::MAX - total(*v), *v))
        .expect("group is non-empty")
}

struct Joined {
    index: usize,
    alignment: Alignment,
}

fn join(pdgs: &[&Pdg], wls: &[Wl], template: usize, member: usize) -> Joined {
    let ctx = Ctx::new(pdgs[template], &wls[template], pdgs[member], &wls[member]);
    Joined {
        index: member,
        alignment: align(&ctx),
    }
}

type ColumnKey = (HoleKind, String);

/// Template value -> (first template site, the value each joined member has there).
fn tabulate(joined: &[Joined]) -> BTreeMap<ColumnKey, (usize, Vec<Option<String>>)> {
    let mut table: BTreeMap<ColumnKey, (usize, Vec<Option<String>>)> = BTreeMap::new();
    for (k, member) in joined.iter().enumerate() {
        for hole in &member.alignment.holes {
            let slot = table
                .entry((hole.kind, hole.a.clone()))
                .or_insert((usize::MAX, vec![None; joined.len()]));
            slot.0 = slot.0.min(hole.sites[0].0);
            slot.1[k].get_or_insert_with(|| hole.b.clone());
        }
    }
    table
}

fn columns_of(joined: &[Joined]) -> Vec<Column> {
    let mut rows: Vec<_> = tabulate(joined).into_iter().collect();
    rows.sort_by_key(|((kind, a), (site, _))| (*site, *kind, a.clone()));
    let mut counts: BTreeMap<HoleKind, usize> = BTreeMap::new();
    rows.into_iter()
        .map(|((kind, template), (_, found))| {
            let n = counts.entry(kind).or_insert(0);
            *n += 1;
            let prefix = crate::align::Hole::var_name(kind, *n - 1);
            let values = std::iter::once(template.clone())
                .chain(
                    found
                        .into_iter()
                        .map(|v| v.unwrap_or_else(|| template.clone())),
                )
                .collect();
            Column {
                kind,
                var: prefix,
                values,
            }
        })
        .collect()
}

fn build(pdgs: &[&Pdg], wls: &[Wl], group: &[usize], params: &Params) -> Option<Cluster> {
    let template = medoid(wls, group);
    let joined: Vec<Joined> = group
        .iter()
        .filter(|v| **v != template)
        .map(|v| join(pdgs, wls, template, *v))
        .filter(|j| j.alignment.coverage_milli >= params.min_coverage_milli)
        .collect();
    (!joined.is_empty()).then(|| Cluster {
        members: std::iter::once(template)
            .chain(joined.iter().map(|j| j.index))
            .collect(),
        coverage_milli: std::iter::once(1000)
            .chain(joined.iter().map(|j| j.alignment.coverage_milli))
            .collect(),
        columns: columns_of(&joined),
    })
}

/// Clusters of structurally parallel functions, largest first. Each function lands in at
/// most one cluster; members must align with the template above the coverage floor.
pub fn cluster(pdgs: &[&Pdg], params: &Params) -> Vec<Cluster> {
    let canonical: Vec<Pdg> = pdgs
        .iter()
        .map(|p| canonicalize(p, &params.normalize))
        .collect();
    let pdgs: Vec<&Pdg> = canonical.iter().collect();
    let pdgs = pdgs.as_slice();
    let wls: Vec<Wl> = pdgs.iter().map(|p| Wl::new(p)).collect();
    let adjacency = similar(pdgs, &wls, &pairs_to_compare(pdgs), params);
    let mut free: BTreeSet<usize> = adjacency.keys().copied().collect();
    let mut found = Vec::new();

    while let Some(seed) = next_seed(&adjacency, &free) {
        let group: Vec<usize> = adjacency[&seed]
            .iter()
            .copied()
            .filter(|v| free.contains(v))
            .chain([seed])
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        match build(pdgs, &wls, &group, params) {
            Some(c) => {
                free.retain(|v| !c.members.contains(v));
                found.push(c);
            }
            None => {
                free.remove(&seed);
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{PdgNode, TyShape};
    use crate::pdgtest::*;

    fn animals() -> Vec<Pdg> {
        vec![
            emit_loop("Dog", "bark"),
            emit_loop("Cat", "meow"),
            emit_loop("Cow", "moo"),
        ]
    }

    fn run(pdgs: &[Pdg], params: &Params) -> Vec<Cluster> {
        let refs: Vec<&Pdg> = pdgs.iter().collect();
        cluster(&refs, params)
    }

    #[test]
    fn three_way_cluster_columns() {
        let clusters = run(&animals(), &Params::default());
        assert_eq!(clusters.len(), 1);
        let c = &clusters[0];
        assert_eq!(c.members, [0, 1, 2]);
        assert_eq!(c.coverage_milli, [1000, 1000, 1000]);
        let cols: Vec<(&str, &[String])> = c
            .columns
            .iter()
            .map(|c| (c.var.as_str(), c.values.as_slice()))
            .collect();
        assert_eq!(cols[0].0, "T0");
        assert_eq!(cols[0].1, ["k::Dog", "k::Cat", "k::Cow"]);
        assert_eq!(cols[1].0, "M0");
        assert_eq!(cols[1].1, ["k::Dog::bark", "k::Cat::meow", "k::Cow::moo"]);
        assert_eq!(cols.len(), 2);
    }

    #[test]
    fn member_below_coverage_rejected() {
        let mut graphs = animals();
        let mut padded = emit_loop("Hen", "cluck");
        padded.nodes.extend([push_call(), push_call()]);
        graphs.push(padded);
        let loose = Params {
            threshold_milli: 500,
            min_coverage_milli: 900,
            ..Params::default()
        };
        let clusters = run(&graphs, &loose);
        assert_eq!(clusters[0].members, [0, 1, 2]);
        let strict_free = Params {
            min_coverage_milli: 700,
            ..loose
        };
        assert_eq!(run(&graphs, &strict_free)[0].members, [0, 1, 2, 3]);
    }

    /// Pads a graph with two calls whose signature class is unique to `tag`, so its two
    /// rarest call classes are not shared with any other function.
    fn with_private_calls(mut pdg: Pdg, tag: usize) -> Pdg {
        for k in 0..2 {
            let mut n = getter_call("Z", "z");
            if let Some(sig) = n.callee_sig.as_mut() {
                sig.params = vec![TyShape::Param(tag * 2 + k + 10)];
            }
            pdg.nodes.push(n);
        }
        pdg
    }

    #[test]
    fn small_corpus_compares_functions_without_a_shared_rare_call() {
        let graphs: Vec<Pdg> = [("Dog", "bark"), ("Cat", "meow")]
            .iter()
            .enumerate()
            .map(|(i, (t, m))| with_private_calls(emit_loop(t, m), i))
            .collect();
        let refs: Vec<&Pdg> = graphs.iter().collect();
        assert!(candidate_pairs(&block_keys(&refs)).is_empty());
        assert_eq!(pairs_to_compare(&refs).len(), 1);
    }

    #[test]
    fn large_corpus_falls_back_to_blocking() {
        let graphs: Vec<Pdg> = (0..=ALL_PAIRS_LIMIT)
            .map(|i| with_private_calls(emit_loop("Dog", "bark"), i))
            .collect();
        let refs: Vec<&Pdg> = graphs.iter().collect();
        assert!(pairs_to_compare(&refs).is_empty());
    }

    #[test]
    fn unrelated_functions_do_not_cluster() {
        let lone = Pdg {
            nodes: vec![
                push_call(),
                getter_call("Dog", "bark"),
                push_call(),
                push_call(),
            ],
            edges: vec![],
        };
        assert!(run(&[emit_loop("Dog", "bark"), lone], &Params::default()).is_empty());
    }

    #[test]
    fn pair_coverage_selects_pairs_the_wl_threshold_rejects() {
        let wl_off = Params {
            threshold_milli: 1001,
            ..Params::default()
        };
        assert!(run(&animals(), &wl_off).is_empty());
        let by_coverage = Params {
            pair_coverage_milli: Some(900),
            ..wl_off
        };
        assert_eq!(run(&animals(), &by_coverage).len(), 1);
    }

    /// `if ok { Err(E::Bad("x")) }` against `if ok { E::Bad }`: same decision, two spellings of the value.
    fn spelled_two_ways() -> Vec<Pdg> {
        use crate::facts::NodeKind::*;
        let lit = |kind: &str, text: &str| PdgNode {
            lit_kind: Some(kind.into()),
            detail: text.into(),
            ..node(Lit)
        };
        let ctor = || PdgNode {
            detail: "ctor".into(),
            ..node(Op)
        };
        let wrapped = Pdg {
            nodes: vec![
                node(Param),
                getter_call("T", "ok"),
                node(Branch),
                lit("str", "\"x\""),
                ctor(),
                ctor(),
                node(Return),
            ],
            edges: vec![
                data(0, 1, 0),
                data(1, 2, 0),
                ctrl(2, 4, 0),
                data(3, 4, 0),
                data(4, 5, 0),
                data(5, 6, 0),
            ],
        };
        let unit = Pdg {
            nodes: vec![
                node(Param),
                getter_call("T", "ok"),
                node(Branch),
                lit("path", "E::Bad"),
                node(Return),
            ],
            edges: vec![data(0, 1, 0), data(1, 2, 0), ctrl(2, 3, 0), data(3, 4, 0)],
        };
        vec![wrapped, unit]
    }

    #[test]
    fn normalize_rules_cluster_functions_that_spell_values_differently() {
        let strict = Params {
            threshold_milli: 900,
            min_coverage_milli: 900,
            ..Params::default()
        };
        assert!(run(&spelled_two_ways(), &strict).is_empty());
        let canonical = Params {
            normalize: Rules::ALL,
            ..strict
        };
        assert_eq!(run(&spelled_two_ways(), &canonical).len(), 1);
    }
}
