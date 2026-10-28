//! Numeric mutation reviews filter presentation without changing captured evidence.
mod observed;
mod retained;

use super::{ReviewStore, mutation_rules, numeric_workflow, outcome, status, summary, workflow};
use crate::clones::{FunctionKey, analysis::Completion, report};
use serde::Serialize;

#[derive(Serialize)]
pub(in crate::clones) struct Report {
    #[serde(flatten)]
    review: numeric_workflow::Presentation<Decision>,
}

#[derive(Serialize)]
pub(super) struct Decision {
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
    numeric_workflow::apply::<observed::Observations>(inputs, ctx, store)
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

impl summary::Decision for Decision {
    fn state(&self) -> &'static str {
        self.outcome.state
    }
}

impl Report {
    pub(in crate::clones) fn hidden_for(&self, key: FunctionKey, rule_id: &str) -> bool {
        self.review.hidden.contains(&(key, rule_id))
    }
    pub(in crate::clones) fn render(&self) -> String {
        self.review.summary.render("Mutation")
    }
}

impl Completion for Report {
    fn permits_success(&self) -> bool {
        self.review.summary.permits_success()
    }
}

impl numeric_workflow::Lifecycle for observed::Observations {
    type Decision = Decision;
    type Report = Report;

    fn enabled(ctx: &report::Context<'_>) -> bool {
        ctx.args.mutation_rules || ctx.args.review_status_requested()
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
        store.mutation_entries.len()
    }

    fn scope(&self) -> summary::Scope {
        summary::Scope {
            requested: self.requested,
            complete: self.complete(),
        }
    }

    fn summarize(decisions: Vec<Decision>, retained: usize, scope: summary::Scope) -> Report {
        Report {
            review: numeric_workflow::Presentation::new(decisions, retained, scope),
        }
    }
}

impl numeric_workflow::Visibility for Decision {
    type Key = (FunctionKey, &'static str);
    const CONTRACT: summary::Contract = summary::Contract {
        schema: "rstyle.mutation-rule-review/1",
        incomplete_scope: "Mutation evidence or review identity scope is incomplete; saved choices cannot establish clean analysis",
    };
    fn hidden_key(&self) -> Option<Self::Key> {
        (self.selected && self.outcome.state == "hidden_exact").then_some((self.key, self.rule_id))
    }
}
