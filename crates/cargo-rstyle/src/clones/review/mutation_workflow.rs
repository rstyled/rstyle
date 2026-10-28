//! Numeric mutation reviews filter presentation without changing captured evidence.
mod observed;
mod retained;

use super::{ReviewStore, mutation_rules, outcome, status, summary, workflow};
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
    hidden: BTreeSet<(FunctionKey, &'static str)>,
}

#[derive(Serialize)]
struct Decision {
    key: FunctionKey,
    rule_id: &'static str,
    selected: bool,
    #[serde(flatten)]
    outcome: outcome::Outcome,
}

pub(super) fn apply(
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
    store: Option<&mut ReviewStore>,
) -> Result<(Report, Vec<status::Record>), String> {
    let Some(store) =
        store.filter(|_| ctx.args.mutation_rules || ctx.args.review_status_requested())
    else {
        return Ok((summarize(Vec::new(), 0, false, false), Vec::new()));
    };
    let observations = observed::collect(inputs, ctx);
    dismiss(store, &observations, ctx)?;
    let decisions = decisions(&observations, store)?;
    let records = retained::records(store, &observations, ctx);

    Ok((
        summarize(
            decisions,
            store.mutation_entries.len(),
            observations.requested,
            observations.complete(),
        ),
        records,
    ))
}

fn decisions(
    observations: &observed::Observations,
    store: &ReviewStore,
) -> Result<Vec<Decision>, String> {
    let mut decisions = Vec::new();
    for rule_id in ["mutation_per_target", "mutated_targets"] {
        let checked =
            mutation_rules::ValidatedStore::new_filtered(&store.mutation_entries, |entry| {
                entry.finding.policy.rule_id() == rule_id
            })
            .map_err(|error| format!("{error:?}"))?;
        for row in &observations.rows {
            if !row.rule(rule_id).is_some_and(|rule| rule.visible) {
                continue;
            }
            let outcome = if row.selected {
                outcome::Outcome::checked(row.checked(rule_id).map(|current| {
                    let lookup: mutation_rules::Lookup<'_> = checked.lookup(&current);
                    lookup.into()
                }))
            } else {
                outcome::Outcome {
                    state: "visible_new",
                    reason: Some("Owner is outside selected review scope".into()),
                }
            };
            decisions.push(Decision {
                key: row.key,
                rule_id,
                selected: row.selected,
                outcome,
            });
        }
    }
    Ok(decisions)
}

fn dismiss(
    store: &mut ReviewStore,
    observations: &observed::Observations,
    ctx: &report::Context<'_>,
) -> Result<(), String> {
    let selector = match (
        &ctx.args.dismiss_mutation_per_target,
        &ctx.args.dismiss_mutated_targets,
    ) {
        (Some(key), None) => Some((key, "mutation_per_target")),
        (None, Some(key)) => Some((key, "mutated_targets")),
        _ => None,
    };
    let Some((selector, rule_id)) = selector else {
        return Ok(());
    };

    if observations.truncated {
        return Err("Mutation review identity budget exhausted; no dismissal recorded".into());
    }
    let row = observations
        .rows
        .iter()
        .find(|row| row.key.to_string() == *selector && row.selected)
        .ok_or("Selected saved-run owner is not a visible selected mutation finding")?;
    let reason = ctx
        .args
        .review_reason
        .as_deref()
        .filter(|reason| !reason.trim().is_empty())
        .ok_or("A dismissal requires a nonempty review reason")?;
    mutation_rules::record_dismissal(
        &mut store.mutation_entries,
        &row.checked(rule_id)?,
        reason.into(),
    )
    .map_err(|error| format!("{error:?}"))?;
    store.schema_version = store.schema_version.max(6);
    Ok(())
}

fn summarize(decisions: Vec<Decision>, retained: usize, requested: bool, complete: bool) -> Report {
    let hidden: BTreeSet<_> = decisions
        .iter()
        .filter(|decision| decision.selected && decision.outcome.state == "hidden_exact")
        .map(|decision| (decision.key, decision.rule_id))
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
            schema: "rstyle.mutation-rule-review/1",
            incomplete_scope: "Mutation evidence or review identity scope is incomplete; saved choices cannot establish clean analysis",
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
    pub(in crate::clones) fn hidden_for(&self, key: FunctionKey, rule_id: &str) -> bool {
        self.hidden.contains(&(key, rule_id))
    }
    pub(in crate::clones) fn render(&self) -> String {
        if !self.summary.requested() {
            return String::new();
        }
        format!(
            "\nMutation reviews: {:?} coverage; {} visible, {} hidden; {} retained entries.",
            self.summary.coverage,
            self.summary.visible_findings,
            self.summary.hidden_findings,
            self.summary.retained_entries
        )
    }
}

impl Completion for Report {
    fn permits_success(&self) -> bool {
        self.summary.permits_success()
    }
}
