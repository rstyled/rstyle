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
//! - Trait-method calls resolve to every impl (and the default body), an over-approximation.
//! - Effects go at most `depth` hops up. Rounds are synchronous, so cycles terminate and
//!   the result does not depend on iteration order.

use crate::facts::{EffectKind, FnFacts};
use crate::rules::Effect;
use crate::suppress;
use std::collections::{BTreeMap, BTreeSet};

/// A resolved call: `facts[target]` may run at `line` of the caller.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Callee {
    pub line: usize,
    pub target: usize,
}

/// `callees[i]` are the resolved calls out of `facts[i]`, sorted by (line, target).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Graph {
    pub callees: Vec<Vec<Callee>>,
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
                .filter_map(|edge| by_id.get(edge.callee.as_str()).map(|t| (edge.line, t)))
                .flat_map(|(line, targets)| {
                    targets.iter().map(move |&target| Callee { line, target })
                })
                .filter(|callee| callee.target != caller)
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

/// What `callee` hands to a caller that calls it at `line`.
fn exported(
    caller: &FnFacts,
    callee: &Callee,
    facts: &[FnFacts],
    current: &[Vec<Effect>],
) -> Vec<Effect> {
    let target = &facts[callee.target];
    let theirs = &current[callee.target];
    let via = Some(target.name.clone());
    let mut out: Vec<Effect> = theirs
        .iter()
        .filter(|e| outward(e.kind))
        .map(|e| Effect {
            line: callee.line,
            via: via.clone(),
            ..e.clone()
        })
        .collect();
    let delegated = caller
        .mutations
        .iter()
        .any(|m| m.delegated && m.line == callee.line);
    if delegated && theirs.iter().any(|e| e.kind == EffectKind::Mutation) {
        out.push(Effect {
            kind: EffectKind::Mutation,
            label: "&mut state".into(),
            line: callee.line,
            via,
        });
    }
    out
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
        .filter(|callee| !inputs.boundary[callee.target])
        .flat_map(|callee| exported(&inputs.facts[index], callee, inputs.facts, current));
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
    fn trait_calls_resolve_to_every_impl() {
        let (mut caller, mut one, mut two) = (sample("caller"), sample("one"), sample("two"));
        caller.edges = vec![edge("krate::Shape::area", 3)];
        one.implements = Some("krate::Shape::area".into());
        two.implements = Some("krate::Shape::area".into());
        let direct = vec![vec![], vec![io(1)], vec![]];
        let facts = [caller, one, two];
        let graph = build(&facts);
        assert_eq!(graph.callees[0].len(), 2);
        let out = run(&facts, &direct, &[false; 3], 1);
        assert_eq!(out[0].len(), 1);
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
}
