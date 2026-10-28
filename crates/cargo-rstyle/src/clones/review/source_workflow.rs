//! Parameter reviews consume completed source measurements, never body proofs.
mod observed;
mod retained;

use super::{ReviewStore, outcome, source_rules, status, summary, workflow};
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
struct Decision {
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
    let requested = ctx.args.source_rules;
    let Some(store) = store.filter(|_| requested || ctx.args.review_status_requested()) else {
        return Ok((summarize(Vec::new(), 0, false, false), Vec::new()));
    };

    let observations = observed::collect(inputs, ctx);
    dismiss(store, &observations, ctx)?;
    let checked = source_rules::ValidatedStore::new(&store.parameter_entries)
        .map_err(|error| format!("{error:?}"))?;
    let decisions = observations
        .rows
        .iter()
        .filter(|row| row.visible)
        .map(|row| decision(row, &checked))
        .collect();
    let records = retained::records(store, &observations, ctx);
    Ok((
        summarize(
            decisions,
            store.parameter_entries.len(),
            requested,
            observations.complete(),
        ),
        records,
    ))
}

fn dismiss(
    store: &mut ReviewStore,
    observations: &observed::Observations,
    ctx: &report::Context<'_>,
) -> Result<(), String> {
    let Some(selector) = ctx.args.dismiss_parameter.as_deref() else {
        return Ok(());
    };
    if observations.truncated {
        return Err("Parameter review identity budget exhausted; no dismissal recorded".into());
    }
    let row = observations
        .rows
        .iter()
        .find(|row| row.key.to_string() == selector && row.visible && row.selected)
        .ok_or("Selected saved-run owner is not a visible selected parameter finding")?;
    let reason = ctx
        .args
        .review_reason
        .as_deref()
        .filter(|reason| !reason.trim().is_empty())
        .ok_or("A dismissal requires a nonempty review reason")?;

    let current = row.checked()?;
    source_rules::record_dismissal(&mut store.parameter_entries, &current, reason.into())
        .map_err(|error| format!("{error:?}"))?;
    store.schema_version = 3;
    Ok(())
}

fn decision(row: &observed::Row, store: &source_rules::ValidatedStore<'_>) -> Decision {
    let outcome =
        outcome::Outcome::checked(row.checked().map(|current| store.lookup(&current).into()));
    Decision {
        key: row.key,
        selected: row.selected,
        outcome,
    }
}

fn summarize(
    decisions: Vec<Decision>,
    retained_entries: usize,
    requested: bool,
    complete: bool,
) -> Report {
    let hidden = decisions
        .iter()
        .filter(|decision| decision.outcome.state == "hidden_exact")
        .map(|decision| decision.key)
        .collect();
    let hidden_selected_findings = decisions
        .iter()
        .filter(|decision| decision.selected && decision.outcome.state == "hidden_exact")
        .count();
    let summary = summary::summarize(
        &decisions,
        retained_entries,
        summary::Scope {
            requested,
            complete,
        },
        summary::Contract {
            schema: "rstyle.parameter-review/1",
            incomplete_scope: "Parameter source assessments or positive finding identities remain unavailable",
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
    pub(in crate::clones) fn hidden_selected_findings(&self) -> usize {
        self.hidden_selected_findings
    }

    pub(in crate::clones) fn hidden_for(&self, key: FunctionKey) -> bool {
        self.hidden.contains(&key)
    }

    pub(in crate::clones) fn render(&self) -> String {
        if !self.summary.requested() {
            return String::new();
        }
        let mut output = format!(
            "\nParameter reviews: {:?} coverage; {} visible, {} hidden; {} retained entries.",
            self.summary.coverage,
            self.summary.visible_findings,
            self.summary.hidden_findings,
            self.summary.retained_entries
        );
        for decision in &self.decisions {
            output.push_str(&format!(
                "\nParameter review {}: {}; selected {}. {}",
                decision.key,
                decision.outcome.state,
                decision.selected,
                decision.outcome.reason.as_deref().unwrap_or("")
            ));
        }
        output
    }
}

impl Completion for Report {
    fn permits_success(&self) -> bool {
        self.summary.permits_success()
    }
}
