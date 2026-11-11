//! What the rules read of a whole run: its authored functions, with their effects
//! completed through the functions macros write and the bodies modules and types run. No
//! rule reports either, since neither was written as a function, but what each does reaches
//! its callers.

use super::records::{self, Measured, Record};
use crate::advisory::generated_effects::Observation;
use crate::config::Config;
use crate::facts::{Edge, FnFacts};
use crate::graph::{self, EffectInput, EffectOwner};
use crate::rules::{self, Effect, EffectEvents};
use crate::wording::Wording;
use crate::{EffectSource, exclusions};
use std::collections::BTreeMap;

/// A run's authored functions and each one's completed effects, by function identity.
pub struct RunInputs {
    pub facts: Vec<FnFacts>,
    pub effects: BTreeMap<String, Vec<Effect>>,
}

enum Row<'a> {
    Authored(&'a FnFacts),
    Generated(&'a Observation, Wording),
}

impl Row<'_> {
    fn identity(&self) -> (&str, Option<&str>) {
        match self {
            Row::Authored(facts) => (&facts.id, facts.implements.as_deref()),
            Row::Generated(observation, _) => match &observation.connections {
                Some(connections) => (&connections.id, connections.implements.as_deref()),
                None => (&observation.name, None),
            },
        }
    }
}

impl EffectInput for Row<'_> {
    fn effect_owner(&self) -> EffectOwner<'_> {
        match self {
            Row::Authored(facts) => facts.effect_owner(),
            Row::Generated(observation, wording) => EffectOwner {
                name: &observation.name,
                mutations: &observation.mutations,
                edges: observation
                    .connections
                    .as_ref()
                    .map_or(&[] as &[Edge], |connections| &connections.edges),
                wording: *wording,
            },
        }
    }
}

impl EffectSource for Row<'_> {
    fn direct_effects(&self, config: &Config) -> Vec<Effect> {
        match self {
            Row::Authored(facts) => rules::effects_of(facts, config),
            Row::Generated(observation, _) => rules::effects_from_events(
                EffectEvents {
                    mutations: &observation.mutations,
                    calls: &observation.calls,
                    effects: &observation.intrinsic_effects,
                },
                config,
            ),
        }
    }

    fn effect_boundary(&self) -> bool {
        matches!(self, Row::Authored(facts) if graph::is_boundary(facts))
    }
}

/// Selects a run's functions as the evaluator does, and completes their effects.
pub fn run_inputs(records: Vec<Record>, config: &Config) -> Result<RunInputs, String> {
    let excluded = exclusions::Matcher::compile(&config.exclude.0)?;
    let mut authored = Vec::new();
    // A unit compiled twice, as a library and its test harness are, writes the same function.
    let mut generated = BTreeMap::new();
    for record in records {
        let observation = match record.measured {
            Measured::Generated(observation) => observation,
            // A body is its author's, but no rule reads it, so its effects count as a macro's do.
            Measured::Body(facts) => Box::new(records::body_observation(*facts)),
            measured => {
                authored.extend(super::authored(Record { measured, ..record }));
                continue;
            }
        };
        if observation.generated_file || !excluded.is_excluded(std::path::Path::new(&record.path)) {
            generated
                .entry(record.identity)
                .or_insert((observation, record.wording));
        }
    }
    let facts = crate::selected_observations(authored, config)?;

    let rows: Vec<Row<'_>> = facts
        .iter()
        .map(Row::Authored)
        .chain(
            generated
                .values()
                .map(|(observation, wording)| Row::Generated(observation, *wording)),
        )
        .collect();
    let graph = graph::build_identified(&rows, Row::identity);
    let effects = crate::all_effects(&rows, &graph, config);
    let effects = facts
        .iter()
        .zip(effects)
        .map(|(fact, effects)| (fact.id.clone(), effects))
        .collect();

    Ok(RunInputs { facts, effects })
}
