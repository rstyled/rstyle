//! Source reviews consume completed numeric measurements, never body proofs.
mod observed;
mod retained;

use super::{ReviewStore, length_rules, outcome, source_rules, status, summary, workflow};
use crate::clones::{FunctionKey, analysis::Completion, report};
use serde::Serialize;
use std::collections::BTreeSet;

pub(in crate::clones) struct Reports {
    pub parameters: Report,
    pub lengths: Report,
}

#[derive(Clone, Copy)]
enum Family {
    Parameter,
    Length,
}

#[derive(Serialize)]
pub(in crate::clones) struct Report {
    #[serde(flatten)]
    summary: summary::Summary,
    hidden_selected_findings: usize,
    decisions: Vec<Decision>,
    #[serde(skip)]
    hidden: BTreeSet<FunctionKey>,
    #[serde(skip)]
    family: Family,
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
) -> Result<(Reports, Vec<status::Record>), String> {
    let requested = ctx.args.source_rules;
    let Some(store) = store.filter(|_| requested || ctx.args.review_status_requested()) else {
        return Ok((
            Reports {
                parameters: summarize(
                    Vec::new(),
                    0,
                    summary::Scope {
                        requested: false,
                        complete: false,
                    },
                    Family::Parameter,
                ),
                lengths: summarize(
                    Vec::new(),
                    0,
                    summary::Scope {
                        requested: false,
                        complete: false,
                    },
                    Family::Length,
                ),
            },
            Vec::new(),
        ));
    };

    let observations = observed::collect(inputs, ctx);
    dismiss(store, &observations, ctx)?;
    let reports = reports(&observations, store, requested)?;
    let records = retained::records(store, &observations, ctx);
    Ok((reports, records))
}

struct ReportInputs<'a> {
    observations: &'a observed::Observations,
    store: &'a ReviewStore,
    requested: bool,
}

fn reports(
    observations: &observed::Observations,
    store: &ReviewStore,
    requested: bool,
) -> Result<Reports, String> {
    let parameters = source_rules::ValidatedStore::new(&store.parameter_entries)
        .map_err(|error| format!("{error:?}"))?;
    let lengths = length_rules::ValidatedStore::new(&store.length_entries)
        .map_err(|error| format!("{error:?}"))?;
    let inputs = ReportInputs {
        observations,
        store,
        requested,
    };
    Ok(Reports {
        parameters: Family::Parameter.report(&inputs, |row| {
            outcome::Outcome::checked(row.checked_parameter().map(|current| {
                let lookup: source_rules::Lookup<'_> = parameters.lookup(&current);
                lookup.into()
            }))
        }),
        lengths: Family::Length.report(&inputs, |row| {
            outcome::Outcome::checked(row.checked_length().map(|current| {
                let lookup: length_rules::Lookup<'_> = lengths.lookup(&current);
                lookup.into()
            }))
        }),
    })
}

fn dismiss(
    store: &mut ReviewStore,
    observations: &observed::Observations,
    ctx: &report::Context<'_>,
) -> Result<(), String> {
    for (family, selector) in [
        (Family::Parameter, ctx.args.dismiss_parameter.as_deref()),
        (Family::Length, ctx.args.dismiss_length.as_deref()),
    ] {
        let Some(selector) = selector else {
            continue;
        };
        if observations.truncated {
            return Err("Source review identity budget exhausted; no dismissal recorded".into());
        }
        let row = observations
            .rows
            .iter()
            .find(|row| row.key.to_string() == selector && family.visible(row) && row.selected)
            .ok_or("Selected saved-run owner is not a visible selected source-rule finding")?;
        let reason = ctx
            .args
            .review_reason
            .as_deref()
            .filter(|reason| !reason.trim().is_empty())
            .ok_or("A dismissal requires a nonempty review reason")?;
        dismiss_one(store, row, family, reason)?;
    }
    Ok(())
}

fn dismiss_one(
    store: &mut ReviewStore,
    row: &observed::Row,
    family: Family,
    reason: &str,
) -> Result<(), String> {
    let result = match family {
        Family::Parameter => source_rules::record_dismissal(
            &mut store.parameter_entries,
            &row.checked_parameter()?,
            reason.into(),
        ),
        Family::Length => length_rules::record_dismissal(
            &mut store.length_entries,
            &row.checked_length()?,
            reason.into(),
        ),
    };
    result.map_err(|error| format!("{error:?}"))?;
    store.schema_version = store.schema_version.max(match family {
        Family::Parameter => 3,
        Family::Length => 4,
    });
    Ok(())
}

impl Family {
    fn name(self) -> &'static str {
        match self {
            Self::Parameter => "Parameter",
            Self::Length => "Length",
        }
    }

    fn visible(self, row: &observed::Row) -> bool {
        match self {
            Self::Parameter => row.parameters.visible,
            Self::Length => row.lengths.visible,
        }
    }

    fn assessment_complete(self, row: &observed::Row) -> bool {
        match self {
            Self::Parameter => row.parameters.complete,
            Self::Length => row.lengths.complete,
        }
    }

    fn complete(self, observations: &observed::Observations) -> bool {
        observations.complete(self)
    }

    fn contract(self) -> summary::Contract {
        match self {
            Self::Parameter => summary::Contract {
                schema: "rstyle.parameter-review/1",
                incomplete_scope: "Parameter source assessments or positive finding identities remain unavailable",
            },
            Self::Length => summary::Contract {
                schema: "rstyle.length-review/1",
                incomplete_scope: "Length source assessments or positive finding identities remain unavailable",
            },
        }
    }

    fn report(
        self,
        inputs: &ReportInputs<'_>,
        mut lookup: impl FnMut(&observed::Row) -> outcome::Outcome,
    ) -> Report {
        let decisions = inputs
            .observations
            .rows
            .iter()
            .filter(|row| self.visible(row))
            .map(|row| Decision {
                key: row.key,
                selected: row.selected,
                outcome: lookup(row),
            })
            .collect();
        let retained = match self {
            Self::Parameter => inputs.store.parameter_entries.len(),
            Self::Length => inputs.store.length_entries.len(),
        };
        summarize(
            decisions,
            retained,
            summary::Scope {
                requested: inputs.requested,
                complete: self.complete(inputs.observations),
            },
            self,
        )
    }
}

fn summarize(
    decisions: Vec<Decision>,
    retained_entries: usize,
    scope: summary::Scope,
    family: Family,
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
    let summary = summary::summarize(&decisions, retained_entries, scope, family.contract());
    Report {
        summary,
        hidden_selected_findings,
        decisions,
        hidden,
        family,
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
            "\n{} reviews: {:?} coverage; {} visible, {} hidden; {} retained entries.",
            self.family.name(),
            self.summary.coverage,
            self.summary.visible_findings,
            self.summary.hidden_findings,
            self.summary.retained_entries
        );
        for decision in &self.decisions {
            output.push_str(&format!(
                "\n{} review {}: {}; selected {}. {}",
                self.family.name(),
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
