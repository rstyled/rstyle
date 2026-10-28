//! Saved density choices filter presentation while preserving numeric evidence.
mod observed;
mod retained;

use super::{ReviewStore, density_rules, numeric_workflow, outcome, status, summary, workflow};
use crate::clones::{FunctionKey, analysis::Completion, report};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Serialize)]
pub(in crate::clones) struct Report {
    #[serde(flatten)]
    summary: summary::Summary,
    hidden_selected_findings: usize,
    decisions: Vec<Decision>,
    #[serde(skip)]
    hidden: BTreeSet<FunctionKey>,
}

#[derive(Serialize)]
pub(super) struct Decision {
    key: FunctionKey,
    selected: bool,
    #[serde(flatten)]
    outcome: outcome::Outcome,
}

pub(super) fn apply(
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
    store: Option<&mut ReviewStore>,
) -> Result<(Report, Vec<status::Record>), String> {
    numeric_workflow::apply::<observed::Observations>(inputs, ctx, store)
}

fn decisions(
    observations: &observed::Observations,
    store: &ReviewStore,
) -> Result<Vec<Decision>, String> {
    let checked = density_rules::ValidatedStore::new(&store.density_entries)
        .map_err(|error| format!("{error:?}"))?;
    Ok(observations
        .rows
        .iter()
        .filter(|row| row.visible)
        .map(|row| {
            let outcome = if row.selected {
                outcome::Outcome::checked(row.checked().map(|current| {
                    let lookup: density_rules::Lookup<'_> = checked.lookup(&current);
                    lookup.into()
                }))
            } else {
                outcome::Outcome {
                    state: "visible_new",
                    reason: Some("Owner is outside selected review scope".into()),
                }
            };
            Decision {
                key: row.key,
                selected: row.selected,
                outcome,
            }
        })
        .collect())
}

fn dismiss(
    store: &mut ReviewStore,
    observations: &observed::Observations,
    ctx: &report::Context<'_>,
) -> Result<(), String> {
    let Some(selector) = &ctx.args.dismiss_density else {
        return Ok(());
    };
    if observations.truncated {
        return Err("Density review identity budget exhausted; no dismissal recorded".into());
    }

    let row = observations
        .rows
        .iter()
        .find(|row| row.key.to_string() == *selector && row.selected)
        .ok_or("Selected saved-run owner is not a visible selected density finding")?;
    let reason = ctx
        .args
        .review_reason
        .as_deref()
        .filter(|reason| !reason.trim().is_empty())
        .ok_or("A dismissal requires a nonempty review reason")?;
    density_rules::record_dismissal(&mut store.density_entries, &row.checked()?, reason.into())
        .map_err(|error| format!("{error:?}"))?;
    store.schema_version = store.schema_version.max(7);
    Ok(())
}

fn summarize(decisions: Vec<Decision>, retained: usize, requested: bool, complete: bool) -> Report {
    let hidden: BTreeSet<_> = decisions
        .iter()
        .filter(|decision| decision.selected && decision.outcome.state == "hidden_exact")
        .map(|decision| decision.key)
        .collect();
    let hidden_selected_findings = hidden.len();
    let summary = summary::summarize(
        &decisions,
        retained,
        summary::Scope {
            requested,
            complete,
        },
        summary::Contract {
            schema: "rstyle.density-review/1",
            incomplete_scope: "Density evidence or review identity scope is incomplete; saved choices cannot establish clean analysis",
        },
    );
    Report {
        summary,
        hidden_selected_findings,
        decisions,
        hidden,
    }
}

impl summary::Decision for Decision {
    fn state(&self) -> &'static str {
        self.outcome.state
    }
}

impl Report {
    pub(in crate::clones) fn hidden_for(&self, key: FunctionKey) -> bool {
        self.hidden.contains(&key)
    }

    pub(in crate::clones) fn render(&self) -> String {
        self.summary.render("Density")
    }
}

impl Completion for Report {
    fn permits_success(&self) -> bool {
        self.summary.permits_success()
    }
}

impl numeric_workflow::Lifecycle for observed::Observations {
    type Decision = Decision;
    type Report = Report;

    fn enabled(ctx: &report::Context<'_>) -> bool {
        ctx.args.effect_density || ctx.args.review_status_requested()
    }

    fn collect(inputs: &workflow::Inputs<'_, '_>, ctx: &report::Context<'_>) -> Self {
        observed::collect(inputs, ctx)
    }

    fn dismiss(&self, store: &mut ReviewStore, ctx: &report::Context<'_>) -> Result<(), String> {
        dismiss(store, self, ctx)
    }

    fn decisions(&self, store: &ReviewStore) -> Result<Vec<Decision>, String> {
        decisions(self, store)
    }

    fn records(&self, store: &ReviewStore, ctx: &report::Context<'_>) -> Vec<status::Record> {
        retained::records(store, self, ctx)
    }

    fn retained(store: &ReviewStore) -> usize {
        store.density_entries.len()
    }

    fn scope(&self) -> summary::Scope {
        summary::Scope {
            requested: self.requested,
            complete: self.complete(),
        }
    }

    fn summarize(decisions: Vec<Decision>, retained: usize, scope: summary::Scope) -> Report {
        summarize(decisions, retained, scope.requested, scope.complete)
    }
}
