//! CI inventories compare completed source assessments without granting evidence.
mod model;
mod observations;
use super::{
    baseline_common::{self, Previous, contexts},
    baseline_io,
    owner_population::{Domain, Population},
    source_workflow::{self, observed::Observations},
    workflow,
};
use crate::clones::report;
pub(in crate::clones) use model::{Baseline, Policy};
use observations::FindingRef;
use std::path::Path;

impl baseline_io::Artifact for Baseline {
    const LABEL: &'static str = "Source baseline";
    fn validate(&self) -> Result<(), String> {
        Baseline::validate(self)
    }
}

pub(in crate::clones) fn load(path: &Path) -> Result<Baseline, String> {
    baseline_io::load(path)
}

pub(in crate::clones) type Report = baseline_common::Report<Baseline>;

pub(super) fn evaluate(
    observations: &Observations,
    reviews: &source_workflow::Reports,
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
) -> Result<Report, String> {
    let header = header(ctx)?;
    let complete = ctx.complete
        && inputs.analysis.source_baseline_valid()
        && observations::complete(observations);

    let current = baseline_common::compare(
        observations::previous(ctx.source_baseline),
        observations
            .rows
            .iter()
            .flat_map(|row| observations::current(row, reviews)),
    );
    Report::evaluate(
        header,
        ctx.source_baseline,
        baseline_common::Assessment {
            complete,
            current,
            retained: ctx
                .source_baseline
                .map_or_else(Vec::new, |baseline| retained(baseline, observations)),
            export_requested: ctx.args.write_source_baseline.is_some(),
        },
        || observations::export(observations),
    )
}

fn header(ctx: &report::Context<'_>) -> Result<Baseline, String> {
    let (policy, exclude) = ctx
        .analysis_config
        .and_then(|config| config.source_baseline_header())
        .ok_or("Source baseline requires source policy")?;
    Baseline::new(Vec::new(), policy, contexts(ctx)?, exclude)
}

fn retained(baseline: &Baseline, observations: &Observations) -> Vec<Previous> {
    let population = Population::new(
        &observations.rows,
        observations.unavailable_scope,
        observations.truncated,
    );
    baseline_common::retained(
        baseline.entries.iter().map(|finding| {
            let (locator, context) = finding.identity();
            (locator, context, finding)
        }),
        population,
        Domain::Source,
        |owner, finding| FindingRef::from(finding).assess(owner),
    )
}
