//! Call graph over `FnFacts` and transitive effect propagation. Pure: no IO, no compiler types.
//!
//! Propagation rules (deliberately conservative, see `exported`):
//! - Only config-free "outward" kinds travel up an edge: io, network, global, time, random.
//!   `unsafe` and `unknown` describe the callee's own body, so callers do not inherit them.
//! - Writes travel only across a delegated `&mut` reborrow (`self.helper()`), as a single
//!   synthetic mutation effect, because that is the one case where the callee writes to
//!   the caller's own state.
//! - A function whose `side_effect_density` is suppressed (with a reason) is a declared
//!   boundary: it absorbs its effects and callers do not inherit them.
//! - Trait-method calls resolve to every impl (and the default body); the caller inherits only
//!   what *every* possible target does, so one expensive impl does not taint all generic callers.
//! - A callee's effects are collapsed to one per kind ("io: open +3 more"), so a caller's
//!   score reflects how many effectful callees it has, not how deep their bodies are.
//! - Effects go at most `depth` hops up. Rounds are synchronous, so cycles terminate and
//!   the result does not depend on iteration order.

use crate::facts::{ArgSource, EffectKind, FnFacts};
use crate::rules::Effect;
use crate::suppress;
use std::collections::{BTreeMap, BTreeSet};

/// A resolved call: one of `targets` runs at `line` of the caller. Several targets
/// mean a trait-method call (any impl, or the default body).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Callee {
    pub line: usize,
    pub targets: Vec<usize>,
    /// Provenance of each argument (see `Edge::args`).
    pub args: Vec<ArgSource>,
}

/// `callees[i]` are the resolved calls out of `facts[i]`, sorted by (line, target).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Graph {
    pub callees: Vec<Vec<Callee>>,
}

impl Graph {
    /// `callers()[i]`: the functions with a resolved call into `facts[i]`, sorted.
    pub fn callers(&self) -> Vec<Vec<usize>> {
        let mut out: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); self.callees.len()];
        for (caller, calls) in self.callees.iter().enumerate() {
            for target in calls.iter().flat_map(|c| &c.targets) {
                out[*target].insert(caller);
            }
        }
        out.into_iter().map(|s| s.into_iter().collect()).collect()
    }
}

pub fn build(facts: &[FnFacts]) -> Graph {
    let mut by_id: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, f) in facts.iter().enumerate() {
        by_id.entry(f.id.as_str()).or_default().push(index);
        if let Some(trait_fn) = &f.implements {
            by_id.entry(trait_fn.as_str()).or_default().push(index);
        }
    }
    let callees = facts
        .iter()
        .enumerate()
        .map(|(caller, f)| {
            let resolved: BTreeSet<Callee> = f
                .edges
                .iter()
                .filter_map(|edge| {
                    let found = by_id.get(edge.callee.as_str())?;
                    let targets: BTreeSet<usize> =
                        found.iter().copied().filter(|&t| t != caller).collect();
                    (!targets.is_empty()).then(|| Callee {
                        line: edge.line,
                        targets: targets.into_iter().collect(),
                        args: edge.args.clone(),
                    })
                })
                .collect();
            resolved.into_iter().collect()
        })
        .collect();
    Graph { callees }
}

/// True for functions that declared themselves a side-effect boundary.
pub fn boundaries(facts: &[FnFacts]) -> Vec<bool> {
    facts
        .iter()
        .map(|f| suppress::allows(&f.preceding_line, "side_effect_density"))
        .collect()
}

fn outward(kind: EffectKind) -> bool {
    matches!(
        kind,
        EffectKind::Io
            | EffectKind::Network
            | EffectKind::Global
            | EffectKind::Time
            | EffectKind::Random
    )
}

/// What one `target` hands to a caller that calls it at `line` (`delegated`: a `&mut self`-style call).
/// `"std::fs::open +3 more"` -> (`"std::fs::open"`, 3); labels without a suffix -> (label, 0).
fn split_more(label: &str) -> (&str, usize) {
    label
        .rsplit_once(" +")
        .and_then(|(base, tail)| {
            let count = tail.strip_suffix(" more")?.parse().ok()?;
            Some((base, count))
        })
        .unwrap_or((label, 0))
}

fn exported_by(target: &FnFacts, theirs: &[Effect], line: usize, delegated: bool) -> Vec<Effect> {
    let via = Some(target.name.clone());

    // One effect per kind: a callee that opens twenty files is one unit of IO for its
    // caller, otherwise orchestrators would score by how deep the call tree is.
    let mut by_kind: BTreeMap<EffectKind, Vec<&Effect>> = BTreeMap::new();
    for effect in theirs.iter().filter(|e| outward(e.kind)) {
        by_kind.entry(effect.kind).or_default().push(effect);
    }
    let mut out: Vec<Effect> = by_kind
        .into_iter()
        .map(|(kind, group)| {
            let (base, _) = split_more(&group[0].label);
            let total: usize = group.iter().map(|e| 1 + split_more(&e.label).1).sum();
            let label = match total - 1 {
                0 => base.to_string(),
                more => format!("{base} +{more} more"),
            };
            Effect {
                kind,
                label,
                line,
                via: via.clone(),
            }
        })
        .collect();
    if delegated && theirs.iter().any(|e| e.kind == EffectKind::Mutation) {
        out.push(Effect {
            kind: EffectKind::Mutation,
            label: "&mut state".into(),
            line,
            via,
        });
    }
    out
}

/// What a call hands to its caller: the effects every possible target exports.
fn exported(
    inputs: &Inputs,
    caller: &FnFacts,
    callee: &Callee,
    current: &[Vec<Effect>],
) -> Vec<Effect> {
    let delegated = caller
        .mutations
        .iter()
        .any(|m| m.delegated && m.line == callee.line);
    let mut per_target = callee.targets.iter().map(|&t| {
        if inputs.boundary[t] {
            return Vec::new();
        }
        exported_by(&inputs.facts[t], &current[t], callee.line, delegated)
    });
    let first = per_target.next().unwrap_or_default();
    let rest: Vec<Vec<Effect>> = per_target.collect();

    first
        .into_iter()
        .filter(|e| {
            rest.iter().all(|other| {
                other
                    .iter()
                    .any(|o| (o.kind, &o.label) == (e.kind, &e.label))
            })
        })
        .collect()
}

/// Everything propagation reads. `direct[i]` and `boundary[i]` belong to `facts[i]`.
pub struct Inputs<'a> {
    pub facts: &'a [FnFacts],
    pub graph: &'a Graph,
    pub direct: &'a [Vec<Effect>],
    pub boundary: &'a [bool],
}

/// One synchronous round for `facts[index]`: its direct effects plus what its callees export.
fn round(inputs: &Inputs, index: usize, current: &[Vec<Effect>]) -> Vec<Effect> {
    let direct = &inputs.direct[index];
    let mut out = direct.clone();
    let mut seen: BTreeSet<(EffectKind, String)> =
        direct.iter().map(|e| (e.kind, e.label.clone())).collect();
    let inherited = inputs.graph.callees[index]
        .iter()
        .flat_map(|callee| exported(inputs, &inputs.facts[index], callee, current));

    for effect in inherited {
        if seen.insert((effect.kind, effect.label.clone())) {
            out.push(effect);
        }
    }
    out
}

/// Direct effects plus effects inherited up to `depth` hops.
pub fn propagate(inputs: &Inputs, depth: usize) -> Vec<Vec<Effect>> {
    let mut current = inputs.direct.to_vec();
    for _ in 0..depth {
        let next: Vec<Vec<Effect>> = (0..inputs.facts.len())
            .map(|i| round(inputs, i, &current))
            .collect();
        if next == current {
            break;
        }
        current = next;
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{Edge, MutationEvent, sample};

    fn edge(callee: &str, line: usize) -> Edge {
        Edge {
            callee: callee.into(),
            line,
            dynamic: false,
            args: vec![],
        }
    }

    fn io(line: usize) -> Effect {
        Effect {
            kind: EffectKind::Io,
            label: "std::fs::write".into(),
            line,
            via: None,
        }
    }

    /// `a -> b -> c`, where only `c` does IO.
    fn chain() -> (Vec<FnFacts>, Vec<Vec<Effect>>) {
        let (mut a, mut b, c) = (sample("a"), sample("b"), sample("c"));
        a.edges = vec![edge("krate::b", 5)];
        b.edges = vec![edge("krate::c", 7)];
        (vec![a, b, c], vec![vec![], vec![], vec![io(2)]])
    }

    fn run(
        facts: &[FnFacts],
        direct: &[Vec<Effect>],
        boundary: &[bool],
        depth: usize,
    ) -> Vec<Vec<Effect>> {
        let graph = build(facts);
        propagate(
            &Inputs {
                facts,
                graph: &graph,
                direct,
                boundary,
            },
            depth,
        )
    }

    #[test]
    fn effects_travel_up_to_depth_hops() {
        let (facts, direct) = chain();
        let none = [false; 3];
        assert!(run(&facts, &direct, &none, 0)[1].is_empty());
        let one = run(&facts, &direct, &none, 1);
        assert_eq!(one[1][0].via.as_deref(), Some("c"));
        assert!(one[0].is_empty(), "two hops away needs depth 2");
        let two = run(&facts, &direct, &none, 2);
        assert_eq!(two[0][0].via.as_deref(), Some("b"));
        assert_eq!(two[0][0].line, 5, "inherited effects sit at the call site");
    }

    #[test]
    fn boundaries_absorb_effects() {
        let (facts, direct) = chain();
        let out = run(&facts, &direct, &[false, false, true], 3);
        assert!(out[1].is_empty() && out[0].is_empty());
    }

    #[test]
    fn cycles_terminate_and_share_effects() {
        let (mut a, mut b) = (sample("a"), sample("b"));
        a.edges = vec![edge("krate::b", 1)];
        b.edges = vec![edge("krate::a", 1)];
        let direct = vec![vec![], vec![io(1)]];
        let out = run(&[a, b], &direct, &[false; 2], 10);
        assert_eq!(out[0].len(), 1);
        assert_eq!(
            out[1].len(),
            1,
            "own io is not double counted via the cycle"
        );
    }

    #[test]
    fn trait_calls_inherit_only_what_every_impl_does() {
        let (mut caller, mut one, mut two) = (sample("caller"), sample("one"), sample("two"));
        caller.edges = vec![edge("krate::Shape::area", 3)];
        one.implements = Some("krate::Shape::area".into());
        two.implements = Some("krate::Shape::area".into());
        let direct = vec![vec![], vec![io(1)], vec![]];
        let facts = [caller, one, two];
        let graph = build(&facts);
        assert_eq!(graph.callees[0].len(), 1);
        assert_eq!(graph.callees[0][0].targets.len(), 2);
        // Only one impl does IO, so a caller that can't know which runs inherits nothing.
        assert!(run(&facts, &direct, &[false; 3], 1)[0].is_empty());
        // When every impl does the same IO, it is inherited.
        let all = vec![vec![], vec![io(1)], vec![io(1)]];
        assert_eq!(run(&facts, &all, &[false; 3], 1)[0].len(), 1);
    }

    #[test]
    fn unsafe_and_unknown_stay_local() {
        let (mut a, b) = (sample("a"), sample("b"));
        a.edges = vec![edge("krate::b", 1)];
        let local = |kind| Effect {
            kind,
            label: "x".into(),
            line: 1,
            via: None,
        };
        let direct = vec![
            vec![],
            vec![local(EffectKind::Unsafe), local(EffectKind::Unknown)],
        ];
        assert!(run(&[a, b], &direct, &[false; 2], 3)[0].is_empty());
    }

    fn write(line: usize, delegated: bool) -> MutationEvent {
        MutationEvent {
            root: "self".into(),
            field_path: String::new(),
            line,
            escapes: !delegated,
            delegated,
        }
    }

    #[test]
    fn delegated_mutation_is_inherited_only_when_callee_writes() {
        let (mut a, mut b, c) = (sample("a"), sample("b"), sample("c"));
        a.edges = vec![edge("krate::b", 4), edge("krate::c", 5)];
        a.mutations = vec![write(4, true), write(5, true)];
        b.mutations = vec![write(9, false)];
        let mutation = Effect {
            kind: EffectKind::Mutation,
            label: "self".into(),
            line: 9,
            via: None,
        };
        let direct = vec![vec![], vec![mutation], vec![]];
        let out = run(&[a, b, c], &direct, &[false; 3], 1);
        assert_eq!(out[0].len(), 1, "only the call to `b` brings a write");
        assert_eq!(out[0][0].line, 4);
    }

    #[test]
    fn a_callee_exports_one_effect_per_kind() {
        let (mut a, b) = (sample("a"), sample("b"));
        a.edges = vec![edge("krate::b", 3)];
        let open = |label: &str| Effect {
            label: label.into(),
            ..io(1)
        };
        let direct = vec![
            vec![],
            vec![
                open("std::fs::open"),
                open("std::fs::read"),
                open("std::fs::write"),
            ],
        ];
        let out = run(&[a, b], &direct, &[false; 2], 1);
        assert_eq!(out[0].len(), 1, "three io effects collapse to one");
        assert_eq!(out[0][0].label, "std::fs::open +2 more");
    }

    #[test]
    fn collapsed_labels_accumulate_counts_across_hops() {
        assert_eq!(split_more("a::b +3 more"), ("a::b", 3));
        assert_eq!(split_more("a::b"), ("a::b", 0));
        let (mut a, mut b, c) = (sample("a"), sample("b"), sample("c"));
        a.edges = vec![edge("krate::b", 1)];
        b.edges = vec![edge("krate::c", 2)];
        let open = |label: &str| Effect {
            label: label.into(),
            ..io(1)
        };
        let direct = vec![
            vec![],
            vec![open("x::read")],
            vec![open("x::open"), open("x::stat")],
        ];
        let out = run(&[a, b, c], &direct, &[false; 3], 2);
        // b: own read + inherited "open +1 more" (distinct labels, both kept).
        assert_eq!(out[1].len(), 2);
        // a: b's two io effects collapse to one: 1 + (1 + 1) = 3 total -> "+2 more".
        assert_eq!(out[0][0].label, "x::read +2 more");
    }
}
