//! Baselines reuse completed compiler diagnostics and the shared CI lifecycle.
mod model;
mod observations;
use super::{
    baseline_common::{self, Assessment, CompilerInventory},
    baseline_io,
};
use crate::clones::{advisory, identity::UnitId, report};
pub(in crate::clones) use model::Baseline;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub(in crate::clones) struct Handle {
    unit: UnitId,
    diagnostic_index: usize,
}

pub(in crate::clones) type Report = baseline_common::Report<Baseline, Handle>;

pub(in crate::clones) fn load(path: &Path) -> Result<Baseline, String> {
    baseline_io::load(path)
}

pub(in crate::clones) fn evaluate(
    advisory: &advisory::Report,
    reviews: &super::compiler_diagnostics::Report,
    ctx: &report::Context<'_>,
) -> Result<Report, String> {
    let rows = advisory.diagnostic_reviews();
    let previous = ctx.compiler_baseline;
    let header = Baseline::new(
        super::compiler_diagnostics::observed::Policy {
            config: advisory.policy(),
            metrics: advisory.metrics,
        },
        baseline_common::contexts(ctx)?,
    )?;
    let complete = ctx.complete
        && advisory.advisory_inputs_complete()
        && ctx.handoff.assessed.len() <= 4096
        && rows.len() <= 4096
        && rows.iter().all(|row| row.snapshot.is_ok());

    Report::evaluate(
        header,
        previous,
        Assessment {
            complete,
            current: baseline_common::compare(
                previous
                    .into_iter()
                    .flat_map(|baseline| &baseline.entries)
                    .map(|finding| (observations::key(finding), finding))
                    .collect(),
                observations::current(
                    &rows,
                    reviews,
                    ctx.analysis_config
                        .and_then(crate::clones::analysis::Config::changed_files),
                ),
            ),
            retained: observations::retained(advisory, ctx, previous),
            export_requested: ctx.args.write_compiler_baseline.is_some(),
        },
        || rows.iter().map(|row| row.snapshot.clone()).collect(),
    )
}
