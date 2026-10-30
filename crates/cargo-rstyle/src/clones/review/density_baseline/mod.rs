//! Explicit CI inventories reuse completed density evidence, never serialized authority.
mod io;
mod model;
use super::{
    baseline_common::{self, Current, Observation, Previous, contexts},
    density_rules,
    density_workflow::{self, observed::Observations},
    numeric_workflow::Retained,
    owner_population::{Domain, Population},
    workflow,
};
use crate::clones::report;
pub(in crate::clones) fn load(path: &Path) -> Result<Baseline, String> {
    io::load(path)
}
pub(in crate::clones) use model::Baseline;
use std::{collections::BTreeMap, path::Path};

pub(in crate::clones) type Report = baseline_common::Report<Baseline>;

pub(super) fn evaluate(
    observations: &Observations,
    reviews: &density_workflow::Report,
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
) -> Result<Report, String> {
    let header = header(ctx)?;
    let complete = observations.complete() && ctx.complete && inputs.effects.baseline_valid();
    let current = compare(observations, reviews, ctx.density_baseline);
    Report::evaluate(
        header,
        ctx.density_baseline,
        baseline_common::Assessment {
            complete,
            current,
            retained: ctx
                .density_baseline
                .map_or_else(Vec::new, |baseline| retained(baseline, observations)),
            export_requested: ctx.args.write_density_baseline.is_some(),
        },
        || export(observations),
    )
}

fn compare(
    observations: &Observations,
    reviews: &density_workflow::Report,
    baseline: Option<&Baseline>,
) -> Vec<Current> {
    let previous: BTreeMap<_, _> = baseline
        .into_iter()
        .flat_map(|baseline| &baseline.entries)
        .map(|snapshot| {
            (
                (&snapshot.locator, &snapshot.context.compilation_target),
                snapshot,
            )
        })
        .collect();

    baseline_common::compare(
        previous,
        observations
            .rows
            .iter()
            .filter(|row| row.visible)
            .map(|row| Observation {
                key: row.key,
                rule_id: None,
                selected: row.selected,
                review_hidden: reviews.hidden_for(row.key),
                finding: row.checked().and_then(|_| row.snapshot()).map(|snapshot| {
                    (
                        (&snapshot.locator, &snapshot.context.compilation_target),
                        snapshot,
                    )
                }),
            }),
    )
}

fn header(ctx: &report::Context<'_>) -> Result<Baseline, String> {
    let settings = ctx
        .effect_config
        .ok_or("Density baseline requires density policy")?;
    Baseline::new(
        Vec::new(),
        density_rules::Policy::from_config(&settings.side_effects),
        contexts(ctx)?,
        settings.exclude.clone(),
    )
}

fn retained(baseline: &Baseline, observations: &Observations) -> Vec<Previous> {
    let population = Population::new(
        &observations.rows,
        observations.unavailable_scope,
        observations.truncated,
    );
    baseline_common::retained(
        baseline
            .entries
            .iter()
            .map(|snapshot| (&snapshot.locator, &snapshot.context, snapshot)),
        population,
        Domain::Density,
        |owner, snapshot| owner.assess(snapshot),
    )
}

fn export(observations: &Observations) -> Result<Vec<density_rules::Snapshot>, String> {
    observations
        .rows
        .iter()
        .filter(|row| row.visible)
        .map(|row| row.snapshot().cloned())
        .collect()
}
