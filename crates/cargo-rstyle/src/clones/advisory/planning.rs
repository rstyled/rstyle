//! Reuse completed compiler diagnostics and the existing resolved propagation graph.
use super::{Collected, Ready, UnitId};
use crate::clones::{FunctionKey, report::Context};
use rstyle_core::{config::Config, facts::FnFacts, graph::Graph, groups::IndexedGroup};
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
    known_functions: usize,
    limitations: &'static [&'static str],
    groups: Vec<Group>,
}

#[derive(Serialize)]
struct Group {
    evaluation: rstyle_core::report::FixGroup,
    members: Vec<FunctionKey>,
    step_owners: Vec<FunctionKey>,
    bundle_owners: Vec<Vec<FunctionKey>>,
}

impl Captured {
    pub(super) fn new(
        ctx: &Context<'_>,
        inputs: &[Collected],
        rows: &[(UnitId, &FnFacts)],
        graph: Graph,
    ) -> Self {
        let keys: BTreeMap<_, _> = ctx
            .handoff
            .unit_contexts
            .iter()
            .zip(inputs)
            .flat_map(|(unit, input)| {
                input
                    .facts
                    .iter()
                    .zip(&input.observations)
                    .map(move |(facts, observation)| {
                        ((unit.id, facts.id.as_str()), observation.key)
                    })
            })
            .collect();
        let owners = rows
            .iter()
            .map(|(unit, facts)| Owner {
                unit: *unit,
                id: facts.id.clone(),
                key: keys[&(*unit, facts.id.as_str())],
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
        let groups =
            rstyle_core::groups::build_indexed(&inputs.facts, &self.graph, &inputs.rules, config)
                .into_iter()
                .map(|group| self.group(group))
                .collect();
        let input_complete = ctx.capture.failures.is_empty()
            && ctx.handoff.failures.is_empty()
            && !ready.is_empty()
            && ready
                .iter()
                .all(|ready| ready.unit.inputs_complete() && ready.unit.propagation_complete);
        Ok(Report {
            schema: "rstyle.compiler-planning/1",
            input_complete,
            known_functions: inputs.facts.len(),
            limitations: &[
                "Advisory core edit footprints and parameter suggestions over captured compiler facts; no rewrite or refactoring-safety authority.",
                "Only captured resolved calls are enumerated; callers outside the selected Cargo scope remain unenumerated even when input_complete is true.",
                "Missing or ambiguous targets are not independent or no-call evidence; counts describe the known graph only.",
                "Generated effect owners have no authored edit coordinates; plans project onto authored owners and remain incomplete when generated source is unavailable.",
                "Core bundle names and types are heuristic display keys, not cross-unit semantic type equivalence.",
                "Raw completed diagnostics drive these plans; reviews and changed-file presentation do not filter the raw planning scope.",
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
            members: keys(group.members),
            step_owners: keys(group.step_owners),
            bundle_owners: group.bundle_owners.into_iter().map(keys).collect(),
        }
    }

    fn inputs<'a>(&self, ready: &'a [Ready]) -> Result<Inputs<'a>, String> {
        let by_id: BTreeMap<_, _> = ready
            .iter()
            .flat_map(|ready| {
                ready
                    .facts
                    .iter()
                    .map(move |facts| ((ready.unit.unit, facts.id.as_str()), facts))
            })
            .collect();
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

        let mut rules: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
        for ready in ready {
            let evaluation = ready
                .unit
                .evaluation
                .as_ref()
                .ok_or("Compiler plans require numeric diagnostics")?;
            for (&owner, diagnostic) in ready.diagnostic_indices.iter().zip(&evaluation.diagnostics)
            {
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
        Ok(Inputs { facts, rules })
    }
}

struct Inputs<'a> {
    facts: Vec<&'a FnFacts>,
    rules: BTreeMap<usize, BTreeSet<String>>,
}

impl Report {
    pub(super) fn render(&self) -> String {
        let mut lines = vec![format!(
            "\nCompiler planning: {} known functions, {} groups; input complete={}. Captured callers only; advisory.",
            self.known_functions,
            self.groups.len(),
            self.input_complete
        )];
        for group in &self.groups {
            lines.extend(rstyle_core::report::group_lines(&group.evaluation));
            let members = group
                .members
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            let steps = group
                .step_owners
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            lines.push(format!(
                "    member owners: {}; step owners: {}",
                members.join(", "),
                steps.join(", ")
            ));
        }
        lines.join("\n")
    }
}
