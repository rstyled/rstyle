//! Existing proposal snapshots feed the shared baseline lifecycle without another mining pass.
mod model;
mod observations;
use super::{
    baseline_common::{self, Assessment, CompilerInventory},
    baseline_io,
};
use crate::clones::{advisory, identity::UnitId, report};
pub(in crate::clones) use model::Baseline;
use serde::Serialize;
use std::{path::Path, rc::Rc};

#[derive(Serialize)]
pub(in crate::clones) struct Handle {
    unit: UnitId,
    candidate_index: usize,
}

pub(in crate::clones) type Report = baseline_common::Report<Baseline, Handle>;

pub(in crate::clones) fn load(path: &Path) -> Result<Baseline, String> {
    baseline_io::load(path)
}

pub(in crate::clones) fn evaluate(
    advisory: &advisory::Report,
    reviews: &super::compiler_patterns::Report,
    ctx: &report::Context<'_>,
) -> Result<Report, String> {
    let rows = advisory.pattern_reviews();
    let previous = ctx.compiler_pattern_baseline;
    let selected = advisory.pattern_selections();
    let header = Baseline::new(
        Rc::clone(advisory.policy()),
        baseline_common::contexts(ctx)?,
    )?
    .with_options(crate::mine_options(&ctx.args.mine));
    let complete = ctx.complete
        && advisory.pattern_inputs_complete()
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
                    .map(|finding| (finding.baseline_key(), finding))
                    .collect(),
                observations::current(&rows, reviews, &selected),
            ),
            retained: observations::retained(&rows, previous),
            export_requested: ctx.args.write_compiler_pattern_baseline.is_some(),
        },
        || rows.iter().map(|row| row.snapshot.clone()).collect(),
    )
}
