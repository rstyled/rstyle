//! Groups similar functions around a template (medoid) instead of chaining pairs, so
//! dissimilar bodies cannot be linked through a middleman (no transitive union-find).

use crate::align::{Alignment, Ctx, HoleKind, align};
use crate::facts::{NodeKind, Pdg};
use crate::normalize::{Rules, canonicalize_or_borrow};
use crate::parallel::mapped;
use crate::wl::{SimilarityKey, Wl, call_class, comparable, similarity_milli};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

/// Blocks larger than this are skipped: their key is too common to say anything.
const BLOCK_CAP: usize = 1000;
/// How many of a function's rarest call classes are used as blocking keys.
const BLOCK_KEYS: usize = 2;
/// Corpora up to this size compare every pair: blocking on the rarest call classes picks
/// classes unique to each function when the corpus is tiny, so parallel functions never meet.
const ALL_PAIRS_LIMIT: usize = 400;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Params {
    pub threshold_milli: u32,
    pub min_coverage_milli: u32,
    /// Experiment: also select pairs whose alignment coverage reaches this (besides WL similarity).
    #[serde(deserialize_with = "serde::Deserialize::deserialize")]
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
    /// Every retained alignment only forwards through distinct, single-use callee holes.
    /// Missing identities, shared calls, repeated holes or other node kinds keep this false.
    pub forwarding_only: bool,
}

fn classes_of(pdg: &Pdg) -> BTreeSet<String> {
    let calls = pdg.nodes.iter().filter(|n| n.kind == NodeKind::Call);
    calls.map(call_class).collect()
}

/// The rarest call classes (across the corpus) of every function.
fn block_keys(pdgs: &[&Pdg]) -> Vec<Vec<String>> {
    let sets: Vec<BTreeSet<String>> = mapped(pdgs, |p| classes_of(p));
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

/// Each pair of functions that share a block, in order, once.
fn candidate_pairs(keys: &[Vec<String>]) -> Vec<(usize, usize)> {
    let mut blocks: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, list) in keys.iter().enumerate() {
        for key in list {
            blocks.entry(key).or_default().push(index);
        }
    }
    let mut pairs: Vec<(usize, usize)> = blocks
        .values()
        .filter(|members| members.len() <= BLOCK_CAP)
        .flat_map(|members| {
            members
                .iter()
                .enumerate()
                .flat_map(|(i, a)| members[i + 1..].iter().map(move |b| (*a, *b)))
        })
        .collect();
    pairs.sort_unstable();
    pairs.dedup();
    pairs
}

/// Every pair for small corpora, pairs that share a blocking key otherwise, in order, once.
fn pairs_to_compare(pdgs: &[&Pdg]) -> Vec<(usize, usize)> {
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
fn similar(pdgs: &[&Pdg], wls: &[Wl], pairs: &[(usize, usize)], params: &Params) -> Adjacency {
    let selected = mapped(pairs, |pair| pair_selected(pdgs, wls, *pair, params));
    let mut adjacency = Adjacency::new();
    for (&(a, b), _) in pairs.iter().zip(selected).filter(|(_, selected)| *selected) {
        adjacency.entry(a).or_default().insert(b);
        adjacency.entry(b).or_default().insert(a);
    }
    adjacency
}

/// The functions not yet in a cluster, and how many such neighbours each has, kept as each one
/// is taken so the next seed needn't count them all again.
struct Seeds<'a> {
    adjacency: &'a Adjacency,
    free: BTreeSet<usize>,
    degree: BTreeMap<usize, usize>,
    /// Each free function with a free neighbour, most of them first, then by index.
    ranked: BTreeSet<(Reverse<usize>, usize)>,
}

impl<'a> Seeds<'a> {
    /// Every function with a neighbour is free, with all of its neighbours.
    fn new(adjacency: &'a Adjacency) -> Self {
        let degree: BTreeMap<usize, usize> = adjacency.iter().map(|(v, n)| (*v, n.len())).collect();
        Seeds {
            adjacency,
            free: adjacency.keys().copied().collect(),
            ranked: degree.iter().map(|(v, d)| (Reverse(*d), *v)).collect(),
            degree,
        }
    }

    /// The free function with the most free neighbours (lowest index on ties), if any has one.
    fn next(&self) -> Option<usize> {
        self.ranked.first().map(|(_, v)| *v)
    }

    /// Takes `v` out of the free functions, if it was one.
    fn take(&mut self, v: usize) {
        if !self.free.remove(&v) {
            return;
        }
        self.ranked.remove(&(Reverse(self.degree[&v]), v));
        let adjacency = self.adjacency;
        let neighbours = adjacency.get(&v).into_iter().flatten();
        neighbours.for_each(|w| self.lower(*w));
    }

    /// One fewer free neighbour for `w`, if it's free.
    fn lower(&mut self, w: usize) {
        if !self.free.contains(&w) {
            return;
        }
        let degree = self
            .degree
            .get_mut(&w)
            .expect("a free function has a degree");
        let ranked = (Reverse(*degree), w);
        *degree -= 1;
        let lowered = (*degree > 0).then_some((Reverse(*degree), w));
        self.ranked.remove(&ranked);
        self.ranked.extend(lowered);
    }
}

/// Members with equal similarity keys, collapsed into one weighted class.
struct Classes {
    /// `(representative, members)` per class, in order of first appearance.
    reps: Vec<(usize, u64)>,
    /// Class of each group member, by group position.
    of_member: Vec<usize>,
}

fn classes(wls: &[Wl], group: &[usize]) -> Classes {
    let mut index: BTreeMap<SimilarityKey<'_>, usize> = BTreeMap::new();
    let mut reps: Vec<(usize, u64)> = Vec::new();
    let mut of_member = Vec::with_capacity(group.len());

    for &v in group {
        let class = *index.entry(wls[v].similarity_key()).or_insert(reps.len());
        if class == reps.len() {
            reps.push((v, 0));
        }
        reps[class].1 += 1;
        of_member.push(class);
    }
    Classes { reps, of_member }
}

/// Total similarity of one member of each class to every other group member. Similarity
/// is symmetric, so each distinct pair of classes is scored once and credited to both.
fn class_totals(wls: &[Wl], reps: &[(usize, u64)]) -> Vec<u64> {
    let score = |a: usize, b: usize| u64::from(similarity_milli(&wls[a], &wls[b]));
    let mut totals = vec![0; reps.len()];
    for (i, &(a, size_a)) in reps.iter().enumerate() {
        let own = if size_a > 1 {
            (size_a - 1) * score(a, a)
        } else {
            0
        };
        let mut total = totals[i] + own;
        for (j, &(b, size_b)) in reps.iter().enumerate().skip(i + 1) {
            let s = score(a, b);
            total += size_b * s;
            totals[j] += size_a * s;
        }
        totals[i] = total;
    }
    totals
}

/// The group member with the highest total similarity to the others (lowest index on
/// ties). Group members are distinct indices.
///
/// Identical members are scored once as a weighted class, as when a sum over many equal
/// subproblems is evaluated once per distinct subproblem and scaled by its multiplicity.
fn medoid(wls: &[Wl], group: &[usize]) -> usize {
    let Classes { reps, of_member } = classes(wls, group);
    let totals = class_totals(wls, &reps);
    group
        .iter()
        .zip(&of_member)
        .min_by_key(|(v, class)| (Reverse(totals[**class]), **v))
        .map(|(v, _)| *v)
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

/// Reuse completed holes and call counts: with no fn-item/literal nodes, each callee
/// hole site is a call. Equality to the smaller call count accounts for every such call.
/// This is proposal policy about shared work, not a body-equivalence or purity proof.
fn forwards_calls(a: &Wl, b: &Wl, alignment: &Alignment) -> bool {
    let calls = a.calls.min(b.calls);
    a.forwarding_shape
        && b.forwarding_shape
        && calls > 0
        && alignment.holes.len() == calls
        && alignment.holes.iter().all(|hole| {
            matches!(hole.kind, HoleKind::Method | HoleKind::FreeFn) && hole.sites.len() == 1
        })
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
        forwarding_only: joined
            .iter()
            .all(|member| forwards_calls(&wls[template], &wls[member.index], &member.alignment)),
    })
}

/// Clusters of structurally parallel functions, largest first. Each function lands in at
/// most one cluster; members must align with the template above the coverage floor.
pub fn cluster(pdgs: &[&Pdg], params: &Params) -> Vec<Cluster> {
    let canonical: Vec<_> = pdgs
        .iter()
        .map(|p| canonicalize_or_borrow(p, &params.normalize))
        .collect();
    let pdgs: Vec<&Pdg> = canonical.iter().map(|p| p.as_ref()).collect();
    let pdgs = pdgs.as_slice();
    let wls: Vec<Wl> = mapped(pdgs, |p| Wl::new(p));
    let adjacency = similar(pdgs, &wls, &pairs_to_compare(pdgs), params);
    let mut seeds = Seeds::new(&adjacency);
    let mut found = Vec::new();

    while let Some(seed) = seeds.next() {
        let group: Vec<usize> = adjacency[&seed]
            .iter()
            .copied()
            .filter(|v| seeds.free.contains(v))
            .chain([seed])
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        match build(pdgs, &wls, &group, params) {
            Some(c) => {
                c.members.iter().for_each(|v| seeds.take(*v));
                found.push(c);
            }
            None => seeds.take(seed),
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{PdgNode, TyShape};
    use crate::pdgtest::*;

    /// The seed counting every free function's free neighbours again gives: the most of them,
    /// then the lowest index.
    fn counted_seed(adjacency: &Adjacency, free: &BTreeSet<usize>) -> Option<usize> {
        free.iter()
            .map(|v| (adjacency[v].intersection(free).count(), *v))
            .filter(|(degree, _)| *degree > 0)
            .min_by_key(|(degree, v)| (Reverse(*degree), *v))
            .map(|(_, v)| v)
    }

    #[test]
    fn seeds_come_in_the_order_counting_every_neighbour_again_gives() {
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut random = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..50 {
            let mut adjacency = Adjacency::new();
            for a in 0..40 {
                for b in (a + 1..40).filter(|_| random() % 6 == 0) {
                    adjacency.entry(a).or_default().insert(b);
                    adjacency.entry(b).or_default().insert(a);
                }
            }
            let mut seeds = Seeds::new(&adjacency);
            let mut free: BTreeSet<usize> = adjacency.keys().copied().collect();
            while let Some(seed) = counted_seed(&adjacency, &free) {
                assert_eq!(seeds.next(), Some(seed));
                // A cluster takes the seed and some of its free neighbours, and can take one twice.
                let neighbours = adjacency[&seed].iter().filter(|v| free.contains(v));
                let some: Vec<usize> = neighbours.copied().filter(|_| random() % 2 == 0).collect();
                for v in some.iter().chain([&seed, &seed]) {
                    free.remove(v);
                    seeds.take(*v);
                }
            }
            assert_eq!(seeds.next(), None);
        }
    }

    #[test]
    fn a_seed_whose_group_makes_no_cluster_is_passed_over() {
        let pdgs = animals();
        assert!(!run(&pdgs, &Params::default()).is_empty());
        let unreachable = Params {
            min_coverage_milli: 1001,
            ..Params::default()
        };
        assert!(run(&pdgs, &unreachable).is_empty());
    }

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

    /// `emit_loop` plus `extra` getter calls on the loop item, each feeding the return.
    fn extended(ty: &str, extra: usize) -> Pdg {
        let mut pdg = emit_loop(ty, "base");
        for k in 0..extra {
            let call = pdg.nodes.len();
            pdg.nodes.push(getter_call(ty, &format!("extra{k}")));
            pdg.edges
                .extend([data(2, call, 0), ctrl(2, call, 0), data(call, 5, k + 1)]);
        }
        pdg
    }

    /// The original definition: every ordered pair scored separately.
    fn reference_medoid(wls: &[Wl], group: &[usize]) -> usize {
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

    #[test]
    fn weighted_class_medoid_matches_pairwise_definition() {
        let pdgs = [
            extended("Dog", 0),
            extended("Cat", 2),
            permuted(&extended("Cow", 2), 3),
            extended("Pig", 1),
            extended("Hen", 4),
            permuted(&extended("Dog", 0), 1),
            extended("Elk", 2),
            extended("Owl", 1),
        ];
        let wls: Vec<Wl> = pdgs.iter().map(Wl::new).collect();
        // Every non-empty subset, in ascending and descending member order.
        for mask in 1u32..1 << pdgs.len() {
            let mut group: Vec<usize> = (0..pdgs.len()).filter(|i| mask >> i & 1 == 1).collect();
            assert_eq!(
                medoid(&wls, &group),
                reference_medoid(&wls, &group),
                "{group:?}"
            );
            group.reverse();
            assert_eq!(
                medoid(&wls, &group),
                reference_medoid(&wls, &group),
                "{group:?}"
            );
        }
        // Names are not labels and node order does not matter: repeated shapes share a class.
        let Classes { reps, of_member } = classes(&wls, &[0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(reps.len(), 4);
        assert_eq!(of_member, [0, 1, 1, 2, 3, 0, 1, 2]);
    }

    #[test]
    fn identical_majority_outweighs_a_distinct_member() {
        let pdgs = [extended("Hen", 4), extended("Dog", 1), extended("Cat", 1)];
        let wls: Vec<Wl> = pdgs.iter().map(Wl::new).collect();
        assert_eq!(medoid(&wls, &[0, 1, 2]), 1);
        assert_eq!(medoid(&wls, &[0, 2, 1]), 1, "ties keep the lowest index");
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
