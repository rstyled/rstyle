//! Fix groups: which violations must be fixed together and which are independent.
//! Pure over facts, the call graph and the final diagnostics.
//!
//! A violation's *footprint* is the set of functions its fix edits: the function itself,
//! plus (for `grouping.caller_rules`, by default `fn_params`) its callers up to
//! `caller_hops`, because changing a signature edits every call site. Violations whose
//! footprints intersect are in one group (union-find). Everything else is independent.
//!
//! Wider footprints (any caller, mutated types) were tried on real code (cargo) and make
//! one hub group out of half the violations, so they are only reported as soft `hints`.

use crate::config::Config;
use crate::facts::FnFacts;
use crate::graph::Graph;
use crate::report::{Diagnostic, FixGroup, GroupFunction, Hint};
use crate::stacking;
use std::borrow::Borrow;
use std::collections::{BTreeMap, BTreeSet};

const MAX_HINTS: usize = 5;
const TYPE_RULES: [&str; 2] = ["mutation_per_target", "mutated_targets"];
const CALLER_HINT_RULE: &str = "side_effect_density";

/// One graph vertex; absent authored facts retain topology without source claims.
#[derive(Clone, Copy)]
pub struct PlanningOwner<'a> {
    pub name: &'a str,
    pub facts: Option<&'a FnFacts>,
}

/// Borrow the existing planning fields without manufacturing numeric observations.
pub trait PlanningInput {
    fn planning_owner(&self) -> PlanningOwner<'_>;
}

impl<F: Borrow<FnFacts>> PlanningInput for F {
    fn planning_owner(&self) -> PlanningOwner<'_> {
        let facts = self.borrow();
        PlanningOwner {
            name: &facts.name,
            facts: Some(facts),
        }
    }
}

impl PlanningInput for PlanningOwner<'_> {
    fn planning_owner(&self) -> PlanningOwner<'_> {
        *self
    }
}

struct Dsu(Vec<usize>);

impl Dsu {
    fn find(&mut self, x: usize) -> usize {
        let parent = self.0[x];

        if parent == x {
            return x;
        }
        let root = self.find(parent);
        self.0[x] = root;
        root
    }

    fn union(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        // Keep the smaller index as root so results do not depend on call order.
        self.0[a.max(b)] = a.min(b);
    }
}

/// Violating functions (by index into `facts`) and the rules they violate, sorted.
fn violators(facts: &[FnFacts], diagnostics: &[Diagnostic]) -> BTreeMap<usize, BTreeSet<String>> {
    let by_location: BTreeMap<(&str, usize, usize), usize> = facts
        .iter()
        .enumerate()
        .map(|(i, f)| ((f.path.as_str(), f.line, f.column), i))
        .collect();
    let mut out: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    for d in diagnostics {
        if let Some(&i) = by_location.get(&(d.path.as_str(), d.line, d.column)) {
            out.entry(i).or_default().insert(d.rule_id.clone());
        }
    }
    out
}

/// `from` plus its callers within `hops` calls.
/// Indices must belong to the same resolved graph and `from` must exist.
pub fn with_callers(from: usize, hops: usize, callers: &[Vec<usize>]) -> BTreeSet<usize> {
    let mut seen: BTreeSet<usize> = BTreeSet::from([from]);
    let mut frontier = vec![from];
    for _ in 0..hops {
        if frontier.is_empty() {
            break;
        }
        let next: Vec<usize> = frontier
            .iter()
            .flat_map(|&f| callers[f].iter().copied())
            .filter(|c| seen.insert(*c))
            .collect();
        frontier = next;
    }
    seen
}

fn footprint(
    index: usize,
    rules: &BTreeSet<String>,
    callers: &[Vec<usize>],
    cfg: &Config,
) -> BTreeSet<usize> {
    with_callers(index, required_caller_hops(rules, cfg), callers)
}

/// Caller reach required by grouping policy, independent of optional hints.
/// Parameter stacking may separately require callsite edits even at zero hops.
pub fn required_caller_hops(rules: &BTreeSet<String>, cfg: &Config) -> usize {
    let reaches_callers = rules
        .iter()
        .any(|r| cfg.grouping.caller_rules.0.contains(r));
    if reaches_callers {
        cfg.grouping.caller_hops
    } else {
        0
    }
}

/// Members ordered callees-first (ties and cycles by source order).
fn stack_order(members: &[usize], graph: &Graph) -> Vec<usize> {
    let set: BTreeSet<usize> = members.iter().copied().collect();
    let mut remaining = set.clone();
    let mut out = Vec::with_capacity(members.len());

    while !remaining.is_empty() {
        let ready = remaining.iter().copied().find(|&m| {
            graph.callees[m]
                .iter()
                .flat_map(|c| &c.targets)
                .all(|t| *t == m || !remaining.contains(t))
        });
        let next = ready.or_else(|| remaining.iter().next().copied());
        let next = next.expect("remaining is non-empty");
        remaining.remove(&next);
        out.push(next);
    }
    out
}

struct Soft {
    types: BTreeSet<String>,
    callers: BTreeSet<usize>,
}

/// What a group's fixes might also touch, without forcing a stack.
fn soft_items<F: PlanningInput>(
    members: &[usize],
    rules: &BTreeMap<usize, BTreeSet<String>>,
    facts: &[F],
    callers: &[Vec<usize>],
) -> Soft {
    let mut soft = Soft {
        types: BTreeSet::new(),
        callers: BTreeSet::new(),
    };
    for &m in members {
        let mine = &rules[&m];
        if TYPE_RULES.iter().any(|r| mine.contains(*r)) {
            soft.types.extend(
                facts[m]
                    .planning_owner()
                    .facts
                    .expect("Diagnostic seed has authored facts")
                    .mutated_types
                    .iter()
                    .cloned(),
            );
        }
        if mine.contains(CALLER_HINT_RULE) {
            soft.callers
                .extend(callers[m].iter().copied().filter(|c| !members.contains(c)));
        }
    }
    soft
}

fn hints_for<F: PlanningInput>(
    group: usize,
    softs: &[Soft],
    owner: &BTreeMap<usize, usize>,
    facts: &[F],
) -> Vec<Hint> {
    let mine = &softs[group];
    let type_hints = mine.types.iter().filter_map(|t| {
        let others = softs
            .iter()
            .enumerate()
            .filter(|(g, s)| *g != group && s.types.contains(t))
            .count();
        (others > 0).then(|| Hint {
            kind: "shared_type".into(),
            item: t.clone(),
            other_groups: others,
        })
    });
    let caller_hints = mine.callers.iter().filter_map(|&c| {
        let mut others: BTreeSet<usize> = softs
            .iter()
            .enumerate()
            .filter(|(_, s)| s.callers.contains(&c))
            .map(|(g, _)| g)
            .collect();
        others.extend(owner.get(&c).copied());
        others.remove(&group);
        (!others.is_empty()).then(|| Hint {
            kind: "shared_caller".into(),
            item: facts[c].planning_owner().name.to_owned(),
            other_groups: others.len(),
        })
    });
    let mut hints: Vec<Hint> = type_hints.chain(caller_hints).collect();

    hints.sort_by(|a, b| {
        (std::cmp::Reverse(a.other_groups), &a.kind, &a.item).cmp(&(
            std::cmp::Reverse(b.other_groups),
            &b.kind,
            &b.item,
        ))
    });
    hints.truncate(MAX_HINTS);
    hints
}

/// Union-find root of every violating function after merging overlapping footprints.
fn roots(
    rules: &BTreeMap<usize, BTreeSet<String>>,
    callers: &[Vec<usize>],
    cfg: &Config,
    size: usize,
) -> Vec<(usize, usize)> {
    let mut dsu = Dsu((0..size).collect());
    for (&index, mine) in rules {
        for item in footprint(index, mine, callers, cfg) {
            dsu.union(index, item);
        }
    }
    rules
        .keys()
        .map(|&index| (index, dsu.find(index)))
        .collect()
}

/// Components of violating functions, each as member indices in source order.
fn components(
    rules: &BTreeMap<usize, BTreeSet<String>>,
    callers: &[Vec<usize>],
    cfg: &Config,
    size: usize,
) -> Vec<Vec<usize>> {
    let mut by_root: BTreeMap<usize, Vec<usize>> = BTreeMap::new();

    for (index, root) in roots(rules, callers, cfg, size) {
        by_root.entry(root).or_default().push(index);
    }
    let mut groups: Vec<Vec<usize>> = by_root.into_values().collect();
    groups.sort_by_key(|members| members[0]);
    groups
}

pub fn build(
    facts: &[FnFacts],
    graph: &Graph,
    diagnostics: &[Diagnostic],
    cfg: &Config,
) -> Vec<FixGroup> {
    let rules = violators(facts, diagnostics);
    build_indexed(facts, graph, &rules, cfg)
        .into_iter()
        .map(|group| group.report)
        .collect()
}

/// A fix group retaining exact owners for contextual adapters.
pub struct IndexedGroup {
    pub report: FixGroup,
    /// Graph owner indices aligned with `report.functions`, in callee-first order.
    pub members: Vec<usize>,
    pub step_owners: Vec<usize>,
    pub bundle_owners: Vec<Vec<usize>>,
    /// Required footprint or signature-callsite owners without authored edit facts.
    pub unavailable_owners: Vec<usize>,
}

/// Group already assessed rules using the caller's exact owner indices.
/// `facts`, graph targets and rule keys must share one index space. This seam does
/// not resolve diagnostics by source location or infer missing callers.
pub fn build_indexed<F: Borrow<FnFacts>>(
    facts: &[F],
    graph: &Graph,
    rules: &BTreeMap<usize, BTreeSet<String>>,
    cfg: &Config,
) -> Vec<IndexedGroup> {
    build_planning(facts, graph, rules, cfg)
}

/// Reuse grouping over the complete graph, including vertices without source facts.
/// Rule keys must identify authored diagnostic seeds. Graph-only vertices can join
/// footprints and hints, but never acquire numeric diagnostics or edit coordinates.
pub fn build_planning<F: PlanningInput>(
    facts: &[F],
    graph: &Graph,
    rules: &BTreeMap<usize, BTreeSet<String>>,
    cfg: &Config,
) -> Vec<IndexedGroup> {
    let callers = graph.callers();
    let groups = components(rules, &callers, cfg, facts.len());
    let softs: Vec<Soft> = groups
        .iter()
        .map(|members| soft_items(members, rules, facts, &callers))
        .collect();
    let owner: BTreeMap<usize, usize> = groups
        .iter()
        .enumerate()
        .flat_map(|(g, members)| members.iter().map(move |&m| (m, g)))
        .collect();
    let ordered: Vec<Vec<usize>> = groups.iter().map(|m| stack_order(m, graph)).collect();
    let plans = stacking::plans_planning(&ordered, rules, facts, graph);

    let reports = GroupReports {
        facts,
        rules,
        softs: &softs,
        owner: &owner,
        callers: &callers,
        config: cfg,
    };
    ordered
        .into_iter()
        .zip(plans)
        .enumerate()
        .map(|(g, (members, plan))| reports.group(g, members, plan))
        .collect()
}

struct GroupReports<'a, F> {
    facts: &'a [F],
    rules: &'a BTreeMap<usize, BTreeSet<String>>,
    softs: &'a [Soft],
    owner: &'a BTreeMap<usize, usize>,
    callers: &'a [Vec<usize>],
    config: &'a Config,
}

impl<F: PlanningInput> GroupReports<'_, F> {
    fn unavailable(&self, members: &[usize], required: Vec<usize>) -> Vec<usize> {
        members
            .iter()
            .flat_map(|index| footprint(*index, &self.rules[index], self.callers, self.config))
            .filter(|index| self.facts[*index].planning_owner().facts.is_none())
            .chain(required)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn function(&self, index: usize) -> GroupFunction {
        let facts = self.facts[index]
            .planning_owner()
            .facts
            .expect("Diagnostic seed has authored facts");
        GroupFunction {
            path: facts.path.clone(),
            line: facts.line,
            name: facts.name.clone(),
            rules: self.rules[&index].iter().cloned().collect(),
        }
    }

    fn group(
        &self,
        g: usize,
        members: Vec<usize>,
        plan: Option<stacking::IndexedPlan>,
    ) -> IndexedGroup {
        let (plan, step_owners, bundle_owners, unavailable) = match plan {
            Some(plan) => (
                Some(plan.report),
                plan.step_owners,
                plan.bundle_owners,
                plan.unavailable_owners,
            ),
            None => (None, Vec::new(), Vec::new(), Vec::new()),
        };
        let unavailable_owners = self.unavailable(&members, unavailable);
        let functions = members.iter().map(|&m| self.function(m)).collect();

        IndexedGroup {
            report: FixGroup {
                id: g + 1,
                functions,
                hints: hints_for(g, self.softs, self.owner, self.facts),
                plan,
            },
            members,
            step_owners,
            bundle_owners,
            unavailable_owners,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{Edge, sample};
    use crate::graph;

    #[test]
    fn caller_reach_bounds_cycles_and_stops_at_a_fixed_point() {
        // 0 <- 1 <- 2 <- 3, with a 1/2 cycle, duplicate edge and isolated 4.
        let callers = vec![vec![1, 1], vec![2], vec![1, 3], vec![], vec![]];
        for (hops, expected) in [(0, vec![0]), (1, vec![0, 1]), (2, vec![0, 1, 2])] {
            assert_eq!(
                with_callers(0, hops, &callers),
                expected.into_iter().collect()
            );
        }
        assert_eq!(
            with_callers(0, usize::MAX, &callers),
            BTreeSet::from([0, 1, 2, 3])
        );
        assert_eq!(with_callers(4, usize::MAX, &callers), BTreeSet::from([4]));
    }

    fn function(name: &str, line: usize, calls: &[&str]) -> FnFacts {
        let mut f = sample(name);
        f.line = line;
        f.edges = calls
            .iter()
            .map(|c| Edge {
                callee: format!("krate::{c}"),
                line: line + 1,
                dynamic: false,
                args: vec![],
            })
            .collect();
        f
    }

    fn flag(f: &FnFacts, rule: &str) -> Diagnostic {
        Diagnostic {
            path: f.path.clone(),
            line: f.line,
            column: f.column,
            rule_id: rule.into(),
            message: String::new(),
            actual: 1,
            limit: 0,
            details: vec![],
        }
    }

    fn run(facts: &[FnFacts], flags: &[(usize, &str)], cfg: &Config) -> Vec<FixGroup> {
        let diagnostics: Vec<Diagnostic> = flags.iter().map(|(i, r)| flag(&facts[*i], r)).collect();
        build(facts, &graph::build(facts), &diagnostics, cfg)
    }

    fn names(group: &FixGroup) -> Vec<&str> {
        group.functions.iter().map(|f| f.name.as_str()).collect()
    }

    fn topology(targets: &[&[usize]]) -> Graph {
        Graph {
            callees: targets
                .iter()
                .map(|targets| {
                    targets
                        .iter()
                        .map(|target| graph::Callee {
                            line: 1,
                            targets: vec![*target],
                            args: vec![],
                        })
                        .collect()
                })
                .collect(),
        }
    }

    fn planning_rules(flags: &[(usize, &str)]) -> BTreeMap<usize, BTreeSet<String>> {
        flags
            .iter()
            .map(|(index, rule)| (*index, BTreeSet::from([(*rule).into()])))
            .collect()
    }

    #[test]
    fn graph_only_shared_caller_groups_authored_signatures_without_fake_edits() {
        let left = function("left", 1, &[]);
        let right = function("right", 10, &[]);
        let owners = [
            left.planning_owner(),
            right.planning_owner(),
            PlanningOwner {
                name: "generated",
                facts: None,
            },
        ];
        let rules = planning_rules(&[(0, "fn_params"), (1, "fn_params")]);
        let groups = build_planning(
            &owners,
            &topology(&[&[], &[], &[0, 1]]),
            &rules,
            &Config::default(),
        );
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].members, [0, 1]);
        assert_eq!(groups[0].step_owners, [0, 1]);
        assert_eq!(groups[0].unavailable_owners, [2]);
        let plan = groups[0].report.plan.as_ref().unwrap();
        assert_eq!(plan.steps.len(), 2);
        assert!(
            plan.steps
                .iter()
                .all(|step| step.signature && step.incoming_sites == 1)
        );
        assert!(plan.bundles.is_empty());
    }

    #[test]
    fn graph_only_shared_density_caller_is_a_hint_without_required_source_edits() {
        let left = function("left", 1, &[]);
        let right = function("right", 10, &[]);
        let owners = [
            left.planning_owner(),
            right.planning_owner(),
            PlanningOwner {
                name: "generated",
                facts: None,
            },
        ];
        let rules = planning_rules(&[(0, "side_effect_density"), (1, "side_effect_density")]);
        let graph = topology(&[&[], &[], &[0, 1]]);
        let groups = build_planning(&owners, &graph, &rules, &Config::default());
        assert_eq!(groups.len(), 2);
        for group in &groups {
            assert!(group.unavailable_owners.is_empty());
            assert!(group.step_owners.is_empty());
            assert_eq!(group.report.hints.len(), 1);
            let hint = &group.report.hints[0];
            assert_eq!(
                (hint.kind.as_str(), hint.item.as_str(), hint.other_groups),
                ("shared_caller", "generated", 1)
            );
        }
    }

    #[test]
    fn caller_footprints_cross_graph_only_bridges_at_existing_hop_limit() {
        let leaf = function("leaf", 1, &[]);
        let top = function("top", 10, &[]);
        let owners = [
            leaf.planning_owner(),
            PlanningOwner {
                name: "bridge",
                facts: None,
            },
            top.planning_owner(),
        ];
        let graph = topology(&[&[], &[0], &[1]]);
        let rules = planning_rules(&[(0, "fn_params"), (2, "fn_length")]);
        let mut config = Config::default();
        let groups = build_planning(&owners, &graph, &rules, &config);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].unavailable_owners, [1]);
        config.grouping.caller_hops = 2;
        let groups = build_planning(&owners, &graph, &rules, &config);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].members, [0, 2]);
        assert_eq!(groups[0].unavailable_owners, [1]);
    }

    #[test]
    fn signature_callsite_edits_remain_required_at_zero_grouping_hops() {
        let leaf = function("leaf", 1, &[]);
        let owners = [
            leaf.planning_owner(),
            PlanningOwner {
                name: "generated",
                facts: None,
            },
            PlanningOwner {
                name: "disconnected",
                facts: None,
            },
        ];
        let graph = topology(&[&[], &[0], &[]]);
        let rules = planning_rules(&[(0, "fn_params")]);
        let mut config = Config::default();
        config.grouping.caller_hops = 0;
        let groups = build_planning(&owners, &graph, &rules, &config);
        assert_eq!(groups[0].unavailable_owners, [1]);
        assert_eq!(groups[0].step_owners, [0]);
    }

    #[test]
    fn indexed_rules_keep_same_location_contexts_separate() {
        let first = function("same", 1, &[]);
        let second = function("same", 1, &[]);
        let facts = [&first, &second];
        let graph = graph::build_resolved(&facts, |_, _| Vec::<usize>::new());
        let rules = BTreeMap::from([(0, BTreeSet::from(["fn_length".into()]))]);
        let groups = build_indexed(&facts, &graph, &rules, &Config::default());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].members, [0]);
        assert_eq!(groups[0].report.functions.len(), 1);
        assert!(groups[0].step_owners.is_empty());
        assert!(groups[0].bundle_owners.is_empty());
        assert!(groups[0].report.plan.is_none());
    }

    #[test]
    fn indexed_footprints_follow_scoped_targets_with_identical_locations() {
        let first = function("same", 1, &[]);
        let second = function("same", 1, &[]);
        let caller = function("caller", 10, &["same"]);
        let facts = [&first, &second, &caller];
        let graph = graph::build_resolved(&facts, |_, _| [0]);
        let rules = BTreeMap::from([
            (0, BTreeSet::from(["fn_params".into()])),
            (1, BTreeSet::from(["fn_params".into()])),
            (2, BTreeSet::from(["fn_length".into()])),
        ]);
        let groups = build_indexed(&facts, &graph, &rules, &Config::default());
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].members, [0, 2]);
        assert_eq!(groups[0].step_owners, [0, 2]);
        assert_eq!(groups[1].members, [1]);
        assert_eq!(groups[1].step_owners, [1]);
        let plan = groups[1].report.plan.as_ref().unwrap();
        assert_eq!(plan.steps[0].incoming_sites, 0);
    }

    #[test]
    fn borrowed_indexed_groups_preserve_existing_reports() {
        let facts = [
            function("caller", 1, &["callee"]),
            function("callee", 10, &[]),
        ];
        let flags = [(0, "fn_length"), (1, "fn_params")];
        let cfg = Config::default();
        let expected = run(&facts, &flags, &cfg);
        let diagnostics: Vec<_> = flags.iter().map(|(i, r)| flag(&facts[*i], r)).collect();
        let rules = violators(&facts, &diagnostics);
        let borrowed: Vec<_> = facts.iter().collect();
        let groups = build_indexed(&borrowed, &graph::build(&facts), &rules, &cfg);
        let owners: Vec<_> = facts.iter().map(PlanningInput::planning_owner).collect();
        let projected = build_planning(&owners, &graph::build(&facts), &rules, &cfg);
        assert_eq!(
            serde_json::to_value(&projected[0].report).unwrap(),
            serde_json::to_value(&groups[0].report).unwrap()
        );
        assert_eq!(projected[0].members, groups[0].members);
        assert_eq!(projected[0].step_owners, groups[0].step_owners);
        assert_eq!(projected[0].bundle_owners, groups[0].bundle_owners);
        assert!(projected[0].unavailable_owners.is_empty());
        assert_eq!(groups[0].members, [1, 0]);
        assert_eq!(groups[0].step_owners, [1, 0]);
        let reports: Vec<_> = groups.into_iter().map(|group| group.report).collect();
        assert_eq!(
            serde_json::to_value(reports).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }

    #[test]
    fn unrelated_violations_are_independent_groups() {
        let facts = [function("a", 1, &[]), function("b", 10, &[])];
        let groups = run(
            &facts,
            &[(0, "fn_length"), (1, "fn_length")],
            &Config::default(),
        );
        assert_eq!(groups.len(), 2);
        assert_eq!((groups[0].id, groups[1].id), (1, 2));
    }

    #[test]
    fn signature_fix_groups_the_function_with_its_callers_callee_first() {
        // caller -> callee; callee's params fix edits caller, which has its own violation.
        let facts = [
            function("caller", 1, &["callee"]),
            function("callee", 10, &[]),
        ];
        let flags = [(0, "fn_length"), (1, "fn_params")];
        let groups = run(&facts, &flags, &Config::default());
        assert_eq!(groups.len(), 1);
        assert_eq!(names(&groups[0]), ["callee", "caller"]);
    }

    #[test]
    fn non_signature_rules_do_not_reach_callers() {
        let facts = [
            function("caller", 1, &["callee"]),
            function("callee", 10, &[]),
        ];
        let flags = [(0, "fn_length"), (1, "side_effect_density")];
        assert_eq!(run(&facts, &flags, &Config::default()).len(), 2);
        let mut cfg = Config::default();
        cfg.grouping
            .caller_rules
            .0
            .push("side_effect_density".into());
        assert_eq!(run(&facts, &flags, &cfg).len(), 1, "configurable");
    }

    #[test]
    fn shared_non_violating_caller_links_two_signature_fixes() {
        let facts = [
            function("hub", 1, &["x", "y"]),
            function("x", 10, &[]),
            function("y", 20, &[]),
        ];
        let flags = [(1, "fn_params"), (2, "fn_params")];
        let groups = run(&facts, &flags, &Config::default());
        assert_eq!(groups.len(), 1, "both fixes edit `hub`");
        assert_eq!(names(&groups[0]), ["x", "y"]);
    }

    #[test]
    fn caller_hops_widen_the_footprint() {
        // top -> mid -> leaf; leaf and top both flagged.
        let facts = [
            function("top", 1, &["mid"]),
            function("mid", 10, &["leaf"]),
            function("leaf", 20, &[]),
        ];
        let flags = [(0, "fn_length"), (2, "fn_params")];
        assert_eq!(run(&facts, &flags, &Config::default()).len(), 2);
        let mut cfg = Config::default();
        cfg.grouping.caller_hops = 2;
        assert_eq!(run(&facts, &flags, &cfg).len(), 1);
    }

    #[test]
    fn cycles_still_order_deterministically() {
        let facts = [function("a", 1, &["b"]), function("b", 10, &["a"])];
        let flags = [(0, "fn_params"), (1, "fn_params")];
        let groups = run(&facts, &flags, &Config::default());
        assert_eq!(names(&groups[0]), ["a", "b"]);
    }

    #[test]
    fn shared_types_are_hints_not_links() {
        let mut one = function("one", 1, &[]);
        let mut two = function("two", 10, &[]);
        one.mutated_types = vec!["krate::Account".into()];
        two.mutated_types = vec!["krate::Account".into()];
        let facts = [one, two];
        let flags = [(0, "mutation_per_target"), (1, "mutated_targets")];
        let groups = run(&facts, &flags, &Config::default());
        assert_eq!(groups.len(), 2, "not stacked");
        assert_eq!(groups[0].hints[0].kind, "shared_type");
        assert_eq!(groups[0].hints[0].other_groups, 1);
        assert_eq!(groups[1].hints[0].item, "krate::Account");
    }

    #[test]
    fn density_callers_are_hints_including_callers_fixed_elsewhere() {
        let facts = [function("caller", 1, &["leaf"]), function("leaf", 10, &[])];
        let flags = [(0, "fn_length"), (1, "side_effect_density")];
        let groups = run(&facts, &flags, &Config::default());
        assert_eq!(groups.len(), 2);
        let hint = &groups[1].hints[0];
        assert_eq!(
            (hint.kind.as_str(), hint.item.as_str()),
            ("shared_caller", "caller")
        );
        assert_eq!(hint.other_groups, 1);
    }

    #[test]
    fn no_violations_no_groups() {
        let facts = [function("a", 1, &[])];
        assert!(run(&facts, &[], &Config::default()).is_empty());
    }
}
