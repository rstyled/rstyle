//! Parameter-refactor stacking plans for `fn_params` fix groups. Pure over facts and the
//! call graph.
//!
//! Changing a signature edits the function and every call site, so the plan is ordered
//! callee signature first, then the callers whose call sites must follow. Two facts make
//! the plan actionable:
//! - *bundles*: parameters that a caller hands to a callee unchanged, several at once
//!   (`root, verbose, depth`). They travel together, so they are a params-struct candidate.
//! - *pass-through* parameters: a caller's parameter that is only ever forwarded to a
//!   function being re-signed. The caller does not need it; the fix belongs in the callee
//!   or an upper layer, and the parameter can leave the caller's signature.

use crate::facts::{ArgSource, FnFacts};
use crate::graph::Graph;
use crate::report::{Bundle, ParamRef, PlanStep, StackPlan};
use std::borrow::Borrow;
use std::collections::{BTreeMap, BTreeSet};

const RULE: &str = "fn_params";

/// One call from `caller` into `callee`, with the argument provenance.
struct Site<'a> {
    caller: usize,
    callee: usize,
    args: &'a [ArgSource],
}

/// Every call site (caller, target) in the graph, in caller then source order.
fn all_sites(graph: &Graph) -> Vec<Site<'_>> {
    graph
        .callees
        .iter()
        .enumerate()
        .flat_map(|(caller, calls)| {
            calls.iter().flat_map(move |call| {
                call.targets.iter().map(move |&callee| Site {
                    caller,
                    callee,
                    args: &call.args,
                })
            })
        })
        .collect()
}

/// `(caller param, callee param)` for every argument that is a bare caller parameter.
fn forwards<F: Borrow<FnFacts>>(site: &Site<'_>, facts: &[F]) -> Vec<(usize, usize)> {
    let caller_params = facts[site.caller].borrow().param_list.len();
    let callee_params = facts[site.callee].borrow().param_list.len();
    site.args
        .iter()
        .enumerate()
        .filter(|(to, _)| *to < callee_params)
        .filter_map(|(to, arg)| match arg {
            ArgSource::Param(from) if *from < caller_params => Some((*from, to)),
            _ => None,
        })
        .collect()
}

type Key = (String, String);

fn key(f: &FnFacts, index: usize) -> Key {
    let p = &f.param_list[index];
    (p.name.clone(), p.ty.clone())
}

/// Itemsets with at least two items and their support, keeping only closed ones: a set is
/// dropped when a strict superset is supported by exactly the same inputs. Candidates are
/// the inputs and their pairwise intersections (one round; enough for plan-sized inputs).
fn closed_sets(inputs: &[BTreeSet<Key>]) -> Vec<(BTreeSet<Key>, usize)> {
    let mut candidates: BTreeSet<BTreeSet<Key>> = inputs.iter().cloned().collect();

    for (i, a) in inputs.iter().enumerate() {
        for b in &inputs[i + 1..] {
            candidates.insert(a.intersection(b).cloned().collect());
        }
    }
    let support = |set: &BTreeSet<Key>| inputs.iter().filter(|i| set.is_subset(i)).count();
    let scored: Vec<(BTreeSet<Key>, usize)> = candidates
        .into_iter()
        .filter(|set| set.len() >= 2)
        .map(|set| {
            let count = support(&set);
            (set, count)
        })
        .collect();
    scored
        .iter()
        .filter(|(set, count)| {
            !scored
                .iter()
                .any(|(other, c)| c == count && other.len() > set.len() && set.is_subset(other))
        })
        .cloned()
        .collect()
}

/// Parameter bundles that travel together through `sites`, biggest first.
fn bundles<F: Borrow<FnFacts>>(sites: &[&Site<'_>], facts: &[F]) -> Vec<(Bundle, Vec<usize>)> {
    let per_site: Vec<BTreeSet<Key>> = sites
        .iter()
        .map(|s| {
            forwards(s, facts)
                .into_iter()
                .map(|(_, to)| key(facts[s.callee].borrow(), to))
                .collect()
        })
        .collect();
    let mut out: Vec<(Bundle, Vec<usize>)> = closed_sets(&per_site)
        .into_iter()
        .map(|(set, count)| {
            let owners: BTreeSet<usize> = sites
                .iter()
                .zip(&per_site)
                .filter(|(_, mine)| set.is_subset(mine))
                .flat_map(|(s, _)| [s.caller, s.callee])
                .collect();
            let carriers: BTreeSet<_> = owners
                .iter()
                .map(|&owner| facts[owner].borrow().name.clone())
                .collect();
            (
                Bundle {
                    params: set
                        .into_iter()
                        .map(|(name, ty)| ParamRef { name, ty })
                        .collect(),
                    sites: count,
                    functions: carriers.into_iter().collect(),
                },
                owners.into_iter().collect(),
            )
        })
        .collect();
    out.sort_by(|(a, _), (b, _)| {
        (
            std::cmp::Reverse(a.params.len()),
            std::cmp::Reverse(a.sites),
            &a.functions,
        )
            .cmp(&(
                std::cmp::Reverse(b.params.len()),
                std::cmp::Reverse(b.sites),
                &b.functions,
            ))
    });
    out
}

/// Parameters of `facts[f]` whose every mention is a forward into `changed` callees.
fn pass_through<F: Borrow<FnFacts>>(f: usize, facts: &[F], sites: &[&Site<'_>]) -> Vec<String> {
    let mut forwarded: BTreeMap<usize, usize> = BTreeMap::new();
    for site in sites.iter().filter(|s| s.caller == f) {
        for (from, _) in forwards(site, facts) {
            *forwarded.entry(from).or_default() += 1;
        }
    }
    facts[f]
        .borrow()
        .param_list
        .iter()
        .enumerate()
        .filter(|(i, p)| p.uses > 0 && forwarded.get(i).is_some_and(|n| *n >= p.uses))
        .map(|(_, p)| p.name.clone())
        .collect()
}

/// Facts plus every call site, computed once for all groups.
struct Ctx<'a, F> {
    facts: &'a [F],
    sites: Vec<Site<'a>>,
}

impl<F: Borrow<FnFacts>> Ctx<'_, F> {
    fn step(&self, index: usize, signature: bool, changed: &BTreeSet<usize>) -> PlanStep {
        let f = self.facts[index].borrow();
        let into_changed: Vec<&Site<'_>> = self
            .sites
            .iter()
            .filter(|s| changed.contains(&s.callee))
            .collect();
        PlanStep {
            path: f.path.clone(),
            line: f.line,
            name: f.name.clone(),
            signature,
            incoming_sites: self.sites.iter().filter(|s| s.callee == index).count(),
            outgoing_sites: into_changed.iter().filter(|s| s.caller == index).count(),
            pass_through: pass_through(index, self.facts, &into_changed),
        }
    }

    /// The plan for one group. `members` are callee-first; those violating `fn_params`
    /// get their signature changed. `None` when no member does.
    fn plan(
        &self,
        members: &[usize],
        violators: &BTreeMap<usize, BTreeSet<String>>,
    ) -> Option<IndexedPlan> {
        let changed: BTreeSet<usize> = members
            .iter()
            .copied()
            .filter(|m| violators[m].contains(RULE))
            .collect();

        if changed.is_empty() {
            return None;
        }
        let feeding: Vec<&Site<'_>> = self
            .sites
            .iter()
            .filter(|s| changed.contains(&s.callee))
            .collect();
        // Callers of a re-signed function and the other members all need edits.
        let touched: BTreeSet<usize> = feeding
            .iter()
            .map(|s| s.caller)
            .chain(members.iter().copied())
            .filter(|c| !changed.contains(c))
            .collect();
        let signatures = members.iter().copied().filter(|m| changed.contains(m));
        let (step_owners, steps) = signatures
            .map(|m| (m, true))
            .chain(touched.into_iter().map(|c| (c, false)))
            .map(|(m, signature)| (m, self.step(m, signature, &changed)))
            .unzip();
        let (bundles, bundle_owners) = bundles(&feeding, self.facts).into_iter().unzip();
        Some(IndexedPlan {
            report: StackPlan { steps, bundles },
            step_owners,
            bundle_owners,
        })
    }
}

/// One plan per group (`None` for groups without a `fn_params` violation).
/// `groups` hold member indices callee-first.
pub fn plans(
    groups: &[Vec<usize>],
    violators: &BTreeMap<usize, BTreeSet<String>>,
    facts: &[FnFacts],
    graph: &Graph,
) -> Vec<Option<StackPlan>> {
    plans_indexed(groups, violators, facts, graph)
        .into_iter()
        .map(|plan| plan.map(|plan| plan.report))
        .collect()
}

/// A plan with exact fact indices, independent of display names and source locations.
pub struct IndexedPlan {
    pub report: StackPlan,
    /// One owner for each report step, in the same order.
    pub step_owners: Vec<usize>,
    /// Exact sorted carrier indices for each bundle; names may be duplicated.
    pub bundle_owners: Vec<Vec<usize>>,
}

/// Reuse parameter planning over borrowed owners in the graph's index space.
/// Group members and rule keys must index `facts`; graph targets use the same roster.
pub fn plans_indexed<F: Borrow<FnFacts>>(
    groups: &[Vec<usize>],
    violators: &BTreeMap<usize, BTreeSet<String>>,
    facts: &[F],
    graph: &Graph,
) -> Vec<Option<IndexedPlan>> {
    let ctx = Ctx {
        facts,
        sites: all_sites(graph),
    };
    groups.iter().map(|g| ctx.plan(g, violators)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{Edge, ParamFacts, sample};
    use crate::graph;

    fn param(name: &str, ty: &str, uses: usize) -> ParamFacts {
        ParamFacts {
            name: name.into(),
            ty: ty.into(),
            uses,
        }
    }

    fn function(name: &str, line: usize, params: Vec<ParamFacts>) -> FnFacts {
        let mut f = sample(name);
        f.line = line;
        f.params = params.len();
        f.param_list = params;
        f
    }

    fn call(from: &mut FnFacts, to: &str, args: Vec<ArgSource>) {
        let line = from.line + 1 + from.edges.len();
        from.edges.push(Edge {
            callee: format!("krate::{to}"),
            line,
            dynamic: false,
            args,
        });
    }

    fn rules(flags: &[(usize, &str)]) -> BTreeMap<usize, BTreeSet<String>> {
        let mut out: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
        for (i, r) in flags {
            out.entry(*i).or_default().insert((*r).into());
        }
        out
    }

    fn one(
        members: &[usize],
        violators: &BTreeMap<usize, BTreeSet<String>>,
        facts: &[FnFacts],
        graph: &Graph,
    ) -> Option<StackPlan> {
        plans(&[members.to_vec()], violators, facts, graph).remove(0)
    }

    use ArgSource::{Const, Field, Other, Param};

    fn abc(uses: usize) -> Vec<ParamFacts> {
        vec![
            param("a", "u8", uses),
            param("b", "bool", uses),
            param("c", "&str", uses),
        ]
    }

    fn keys(names: &[&str]) -> BTreeSet<Key> {
        names
            .iter()
            .map(|n| (n.to_string(), "t".to_string()))
            .collect()
    }

    #[test]
    fn indexed_plans_keep_duplicate_display_names_as_distinct_owners() {
        let callee = function("same", 1, abc(1));
        let mut caller = function("same", 1, abc(1));
        call(&mut caller, "same", vec![Param(0), Param(1), Param(2)]);
        let facts = [&callee, &caller];
        let graph = graph::build_resolved(&facts, |_, _| [0]);
        let mut plans = plans_indexed(&[vec![0]], &rules(&[(0, RULE)]), &facts, &graph);
        let plan = plans.remove(0).unwrap();
        assert_eq!(plan.step_owners, [0, 1]);
        assert_eq!(plan.bundle_owners, [vec![0, 1]]);
        assert_eq!(plan.report.bundles[0].functions, ["same"]);
        assert_eq!(plan.report.bundles[0].sites, 1);
        assert_eq!(plan.report.steps[1].pass_through, ["a", "b", "c"]);
    }

    #[test]
    fn closed_sets_drop_subsets_with_equal_support() {
        let sets = [keys(&["a", "b", "c"]), keys(&["a", "b", "c"])];
        let found = closed_sets(&sets);
        assert_eq!(found, vec![(keys(&["a", "b", "c"]), 2)]);
    }

    #[test]
    fn closed_sets_keep_a_smaller_set_with_more_support() {
        let sets = [
            keys(&["a", "b", "c"]),
            keys(&["a", "b", "d"]),
            keys(&["a", "b"]),
        ];
        let found = closed_sets(&sets);
        assert!(found.contains(&(keys(&["a", "b"]), 3)), "{found:?}");
        assert!(found.contains(&(keys(&["a", "b", "c"]), 1)), "{found:?}");
        assert_eq!(found.len(), 3);
    }

    #[test]
    fn closed_sets_ignore_singletons_and_empty_input() {
        assert!(closed_sets(&[keys(&["a"]), keys(&["a"])]).is_empty());
        assert!(closed_sets(&[]).is_empty());
    }

    #[test]
    fn forwards_pair_caller_and_callee_positions() {
        let callee = function("callee", 1, abc(1));
        let mut caller = function("caller", 10, abc(1));
        call(&mut caller, "callee", vec![Param(2), Const, Param(0)]);
        let facts = [callee, caller];
        let graph = graph::build(&facts);
        let sites = all_sites(&graph);
        assert_eq!(forwards(&sites[0], &facts), vec![(2, 0), (0, 2)]);
    }

    #[test]
    fn forwards_ignore_out_of_range_indices() {
        let callee = function("callee", 1, abc(1));
        let mut caller = function("caller", 10, vec![param("a", "u8", 1)]);
        call(
            &mut caller,
            "callee",
            vec![Param(5), Param(0), Other, Field, Const],
        );
        let facts = [callee, caller];
        let graph = graph::build(&facts);
        let sites = all_sites(&graph);
        assert_eq!(forwards(&sites[0], &facts), vec![(0, 1)]);
    }

    #[test]
    fn no_plan_without_a_fn_params_violation() {
        let facts = [function("lonely", 1, abc(1))];
        let graph = graph::build(&facts);
        let flags = rules(&[(0, "fn_length")]);
        assert!(one(&[0], &flags, &facts, &graph).is_none());
    }

    #[test]
    fn plan_lists_callee_signature_then_call_site_edits() {
        let callee = function("callee", 1, abc(1));
        let mut mid = function("mid", 10, abc(1));
        call(&mut mid, "callee", vec![Param(0), Param(1), Param(2)]);
        let mut top = function("top", 20, vec![]);
        call(&mut top, "mid", vec![Const, Const, Const]);
        call(&mut top, "mid", vec![Const, Const, Const]);
        let facts = [callee, mid, top];
        let graph = graph::build(&facts);
        let flags = rules(&[(0, RULE)]);
        let p = one(&[0], &flags, &facts, &graph).unwrap();
        let steps: Vec<(&str, bool, usize, usize)> = p
            .steps
            .iter()
            .map(|s| {
                (
                    s.name.as_str(),
                    s.signature,
                    s.incoming_sites,
                    s.outgoing_sites,
                )
            })
            .collect();
        // `top` calls `mid`, not the re-signed `callee`: outside this plan.
        assert_eq!(steps, [("callee", true, 1, 0), ("mid", false, 2, 1)]);
    }

    #[test]
    fn plan_orders_violating_callers_after_their_callees() {
        let leaf = function("leaf", 1, abc(1));
        let mut mid = function("mid", 10, abc(1));
        call(&mut mid, "leaf", vec![Param(0), Param(1), Param(2)]);
        let mut top = function("top", 20, abc(1));
        call(&mut top, "mid", vec![Param(0), Param(1), Param(2)]);
        let facts = [leaf, mid, top];
        let graph = graph::build(&facts);
        let flags = rules(&[(2, RULE), (1, RULE), (0, RULE)]);
        let p = one(&[0, 1, 2], &flags, &facts, &graph).unwrap();
        let order: Vec<&str> = p.steps.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(order, ["leaf", "mid", "top"]);
        assert!(p.steps.iter().all(|s| s.signature));
    }

    #[test]
    fn bundle_is_params_forwarded_together_by_a_caller() {
        let callee = function("callee", 1, abc(1));
        let mut mid = function("mid", 10, abc(1));
        call(&mut mid, "callee", vec![Param(0), Param(1), Param(2)]);
        let facts = [callee, mid];
        let graph = graph::build(&facts);
        let p = one(&[0], &rules(&[(0, RULE)]), &facts, &graph).unwrap();
        assert_eq!(p.bundles.len(), 1);
        let names: Vec<&str> = p.bundles[0]
            .params
            .iter()
            .map(|x| x.name.as_str())
            .collect();
        assert_eq!(names, ["a", "b", "c"]);
        assert_eq!(p.bundles[0].sites, 1);
        assert_eq!(p.bundles[0].functions, ["callee", "mid"]);
    }

    #[test]
    fn constants_and_fields_do_not_form_a_bundle() {
        let callee = function("callee", 1, abc(1));
        let mut caller = function("caller", 10, vec![param("x", "u8", 1)]);
        call(&mut caller, "callee", vec![Const, Field, Param(0)]);
        let facts = [callee, caller];
        let graph = graph::build(&facts);
        let p = one(&[0], &rules(&[(0, RULE)]), &facts, &graph).unwrap();
        assert!(p.bundles.is_empty());
    }

    #[test]
    fn bundle_support_counts_sites_across_callers() {
        let callee = function("callee", 1, abc(1));
        let mut first = function("one", 10, abc(1));
        call(&mut first, "callee", vec![Param(0), Param(1), Param(2)]);
        let mut two = function("two", 20, abc(1));
        call(&mut two, "callee", vec![Param(0), Param(1), Const]);
        let facts = [callee, first, two];
        let graph = graph::build(&facts);
        let p = one(&[0], &rules(&[(0, RULE)]), &facts, &graph).unwrap();
        let sizes: Vec<(usize, usize)> = p
            .bundles
            .iter()
            .map(|b| (b.params.len(), b.sites))
            .collect();
        assert_eq!(sizes, [(3, 1), (2, 2)]);
    }

    #[test]
    fn param_only_forwarded_is_pass_through() {
        let callee = function("callee", 1, abc(1));
        let params = vec![
            param("a", "u8", 1),
            param("b", "bool", 2),
            param("c", "&str", 1),
        ];
        let mut mid = function("mid", 10, params);
        call(&mut mid, "callee", vec![Param(0), Param(1), Other]);
        let facts = [callee, mid];
        let graph = graph::build(&facts);
        let p = one(&[0], &rules(&[(0, RULE)]), &facts, &graph).unwrap();
        // `a` is only forwarded; `b` is read twice but forwarded once; `c` is never forwarded.
        assert_eq!(p.steps[1].pass_through, ["a"]);
    }

    #[test]
    fn unused_param_is_not_pass_through() {
        let callee = function("callee", 1, abc(1));
        let mut mid = function("mid", 10, abc(0));
        call(&mut mid, "callee", vec![Param(0), Param(1), Param(2)]);
        let facts = [callee, mid];
        let graph = graph::build(&facts);
        let p = one(&[0], &rules(&[(0, RULE)]), &facts, &graph).unwrap();
        assert!(p.steps[1].pass_through.is_empty());
    }

    #[test]
    fn plan_is_deterministic_under_caller_order() {
        let callee = function("callee", 1, abc(1));
        let mut b = function("b", 30, abc(1));
        call(&mut b, "callee", vec![Param(0), Param(1), Param(2)]);
        let mut a = function("a", 20, abc(1));
        call(&mut a, "callee", vec![Param(0), Param(1), Param(2)]);
        let facts = [callee, a, b];
        let graph = graph::build(&facts);
        let p = one(&[0], &rules(&[(0, RULE)]), &facts, &graph).unwrap();
        let order: Vec<&str> = p.steps.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(order, ["callee", "a", "b"]);
        assert_eq!(p.bundles[0].sites, 2);
    }
}
