//! Reuse completed compiler diagnostics and the existing resolved propagation graph.
use super::{Collected, Ready, UnitId};
use crate::clones::{FunctionKey, report::Context};
use rstyle_core::{
    config::Config,
    graph::Graph,
    groups::{IndexedGroup, PlanningOwner},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

struct Owner {
    unit: UnitId,
    id: String,
    key: FunctionKey,
}

pub(super) struct Captured {
    graph: Graph,
    owners: Vec<Owner>,
}

#[derive(Serialize)]
pub(super) struct Report {
    schema: &'static str,
    input_complete: bool,
    required_edits_complete: bool,
    known_functions: usize,
    known_call_graph_functions: usize,
    call_graph_complete: bool,
    limitations: &'static [&'static str],
    groups: Vec<Group>,
}

#[derive(Serialize)]
struct Group {
    evaluation: rstyle_core::report::FixGroup,
    members: Vec<FunctionKey>,
    step_owners: Vec<FunctionKey>,
    bundle_owners: Vec<Vec<FunctionKey>>,
    unavailable_owners: Vec<FunctionKey>,
}

impl Captured {
    pub(super) fn new<'a>(
        ctx: &Context<'_>,
        inputs: &[Collected],
        rows: impl Iterator<Item = (UnitId, &'a str)>,
        graph: Graph,
    ) -> Self {
        let keys: BTreeMap<_, _> =
            ctx.handoff
                .unit_contexts
                .iter()
                .zip(inputs)
                .flat_map(|(unit, input)| {
                    let authored = input.facts.iter().zip(&input.observations).map(
                        move |(facts, observation)| ((unit.id, facts.id.as_str()), observation.key),
                    );
                    let generated = input.generated_effects.iter().filter_map(move |row| {
                        Some(((unit.id, row.connections()?.id.as_str()), row.key))
                    });
                    authored.chain(generated)
                })
                .collect();
        let owners = rows
            .map(|(unit, id)| Owner {
                unit,
                id: id.to_owned(),
                key: keys[&(unit, id)],
            })
            .collect();
        Self { graph, owners }
    }

    pub(super) fn evaluate(
        self,
        ctx: &Context<'_>,
        ready: &[Ready],
        config: &Config,
    ) -> Result<Report, String> {
        let inputs = self.inputs(ready)?;
        let groups: Vec<_> =
            rstyle_core::groups::build_planning(&inputs.facts, &self.graph, &inputs.rules, config)
                .into_iter()
                .map(|group| self.group(group))
                .collect();

        let captured =
            ctx.capture.failures.is_empty() && ctx.handoff.failures.is_empty() && !ready.is_empty();
        let call_graph_complete =
            captured && ready.iter().all(|ready| ready.unit.propagation_complete);
        let input_complete = captured
            && ready.iter().all(|ready| {
                ready.unit.inputs_complete()
                    && ready.unit.full_diagnostics_complete()
                    && ready.unit.propagation_complete
            });
        let required_edits_complete = input_complete
            || (captured
                && ready.iter().all(|ready| ready.unit.diagnostics_complete())
                && inputs.required_edits_complete(&groups, config, call_graph_complete));

        Ok(Report {
            schema: "rstyle.compiler-planning/1",
            input_complete,
            required_edits_complete,
            known_functions: inputs
                .facts
                .iter()
                .filter(|owner| owner.facts.is_some())
                .count(),
            known_call_graph_functions: inputs.facts.len(),
            call_graph_complete,
            limitations: &[
                "Advisory core edit footprints and parameter suggestions over captured compiler facts; no rewrite or refactoring-safety authority.",
                "Only captured resolved calls are enumerated; callers outside the selected Cargo scope remain unenumerated even when input_complete is true.",
                "Missing or ambiguous targets are not independent or no-call evidence; counts describe the known graph only.",
                "Generated owners retain known caller topology but have no authored edit coordinates or parameter facts; required missing edits are listed by owner, without fabricated steps or forwarding suggestions.",
                "required_edits_complete describes requested edit worklists only; missing caller graphs and optional hints remain outside that status. Parameter plans still require caller inputs.",
                "Core bundle names and types are heuristic display keys, not cross-unit semantic type equivalence.",
                "Requested completed diagnostics drive these plans; reviews and changed-file presentation do not filter planning seeds.",
            ],
            groups,
        })
    }

    fn group(&self, group: IndexedGroup) -> Group {
        let keys = |indices: Vec<usize>| {
            indices
                .into_iter()
                .map(|index| self.owners[index].key)
                .collect()
        };
        Group {
            evaluation: group.report,
            unavailable_owners: keys(group.unavailable_owners),
            members: keys(group.members),
            step_owners: keys(group.step_owners),
            bundle_owners: group.bundle_owners.into_iter().map(keys).collect(),
        }
    }

    fn inputs<'a>(&self, ready: &'a [Ready]) -> Result<Inputs<'a>, String> {
        let by_id = planning_owners(ready);
        let indices: BTreeMap<_, _> = self
            .owners
            .iter()
            .enumerate()
            .map(|(index, owner)| ((owner.unit, owner.id.as_str()), index))
            .collect();
        let facts = self
            .owners
            .iter()
            .map(|owner| {
                by_id
                    .get(&(owner.unit, owner.id.as_str()))
                    .copied()
                    .ok_or_else(|| "Completed compiler planning owner is missing".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;

        let rules = selected_rules(ready, &indices)?;
        Ok(Inputs { facts, rules })
    }
}

fn planning_owners(ready: &[Ready]) -> BTreeMap<(UnitId, &str), PlanningOwner<'_>> {
    ready
        .iter()
        .flat_map(|ready| {
            let authored = ready.facts.iter().map(move |facts| {
                (
                    (ready.unit.unit, facts.id.as_str()),
                    PlanningOwner {
                        name: &facts.name,
                        facts: Some(facts),
                    },
                )
            });
            let generated = ready.unit.generated_effects.iter().filter_map(move |row| {
                Some((
                    (ready.unit.unit, row.connections()?.id.as_str()),
                    PlanningOwner {
                        name: row.owner().name,
                        facts: None,
                    },
                ))
            });
            authored.chain(generated)
        })
        .collect()
}

fn selected_rules(
    ready: &[Ready],
    indices: &BTreeMap<(UnitId, &str), usize>,
) -> Result<BTreeMap<usize, BTreeSet<String>>, String> {
    let mut rules: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    for ready in ready {
        let evaluation = ready
            .unit
            .evaluation
            .as_ref()
            .ok_or("Compiler plans require numeric diagnostics")?;
        for (diagnostic_index, (&owner, diagnostic)) in ready
            .diagnostic_indices
            .iter()
            .zip(&evaluation.diagnostics)
            .enumerate()
        {
            if !ready.unit.diagnostic_requested(diagnostic_index) {
                continue;
            }
            let fact = &ready.facts[owner];
            let index = indices
                .get(&(ready.unit.unit, fact.id.as_str()))
                .ok_or("Completed compiler diagnostic has no planning owner")?;
            rules
                .entry(*index)
                .or_default()
                .insert(diagnostic.rule_id.clone());
        }
    }
    Ok(rules)
}

struct Inputs<'a> {
    facts: Vec<PlanningOwner<'a>>,
    rules: BTreeMap<usize, BTreeSet<String>>,
}

impl Inputs<'_> {
    fn required_edits_complete(
        &self,
        groups: &[Group],
        config: &Config,
        graph_complete: bool,
    ) -> bool {
        if graph_complete
            && groups
                .iter()
                .all(|group| group.unavailable_owners.is_empty())
        {
            return true;
        }
        self.rules
            .values()
            .all(|rules| rstyle_core::groups::required_caller_hops(rules, config) == 0)
            && groups.iter().all(|group| group.evaluation.plan.is_none())
    }
}

impl Report {
    pub(super) fn groups(&self) -> impl Iterator<Item = &rstyle_core::report::FixGroup> {
        self.groups.iter().map(|group| &group.evaluation)
    }

    pub(super) fn input_complete(&self) -> bool {
        self.input_complete
    }

    pub(super) fn required_edits_complete(&self) -> bool {
        self.required_edits_complete
    }

    pub(super) fn render(&self) -> String {
        let mut lines = vec![format!(
            "\nCompiler planning: {} known functions, {} groups; input complete={}, required edits complete={}. Captured callers only; advisory.",
            self.known_functions,
            self.groups.len(),
            self.input_complete,
            self.required_edits_complete
        )];
        lines.push(format!(
            "    {} known call graph functions; call graph complete={}",
            self.known_call_graph_functions, self.call_graph_complete,
        ));
        lines.extend(self.groups.iter().map(Group::render));
        lines.join("\n")
    }
}

impl Group {
    fn render(&self) -> String {
        let keys = |owners: &[FunctionKey]| {
            owners
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        };
        let unavailable = if self.unavailable_owners.is_empty() {
            String::new()
        } else {
            format!(
                "    required edit data unavailable for owners: {}\n",
                keys(&self.unavailable_owners)
            )
        };
        format!(
            "{}\n{unavailable}    member owners: {}; step owners: {}",
            rstyle_core::report::group_lines(&self.evaluation).join("\n"),
            keys(&self.members),
            keys(&self.step_owners),
        )
    }
}
