//! Source reviews consume completed numeric measurements, never body proofs.
pub(super) mod observed;
pub(super) mod retained;

use super::{
    ReviewStore, length_rules, outcome, source_rules, spacing_rules, status, summary, workflow,
};
use crate::clones::{FunctionKey, analysis::Completion, report};
use serde::Serialize;
use std::collections::BTreeSet;

pub(in crate::clones) struct Reports {
    pub parameters: Report,
    pub lengths: Report,
    pub spacings: Report,
    pub baseline: Option<super::source_baseline::Report>,
}

#[derive(Clone, Copy)]
pub(super) enum Family {
    Parameter,
    Length,
    Spacing,
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
    let baseline_requested =
        ctx.args.source_baseline.is_some() || ctx.args.write_source_baseline.is_some();
    let store = store.filter(|_| requested || ctx.args.review_status_requested());
    if store.is_none() && !baseline_requested {
        return Ok((Reports::not_requested(), Vec::new()));
    }

    let observations = observed::collect(inputs, ctx);
    let (mut reports, records) = if let Some(store) = store {
        dismiss(store, &observations, ctx)?;
        (
            reports(&observations, store, requested)?,
            retained::records(store, &observations, ctx),
        )
    } else {
        (Reports::not_requested(), Vec::new())
    };

    if baseline_requested {
        reports.baseline = Some(super::source_baseline::evaluate(
            &observations,
            &reports,
            inputs,
            ctx,
        )?);
    }
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
    let spacings = spacing_rules::ValidatedStore::new(&store.spacing_entries)
        .map_err(|error| format!("{error:?}"))?;

    let inputs = ReportInputs {
        observations,
        store,
        requested,
    };
    Ok(Reports {
        baseline: None,
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
        spacings: Family::Spacing.report(&inputs, |row| {
            outcome::Outcome::checked(row.checked_spacing().map(|current| {
                let lookup: spacing_rules::Lookup<'_> = spacings.lookup(&current);
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
        (Family::Spacing, ctx.args.dismiss_spacing.as_deref()),
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

// rstyle-allow(side_effect_density): Review mutation boundary records one choice and advances its required schema together.
fn dismiss_one(
    store: &mut ReviewStore,
    row: &observed::Row,
    family: Family,
    reason: &str,
) -> Result<(), String> {
    let required_schema = record_choice(store, row, family, reason)?;
    store.schema_version = store.schema_version.max(required_schema);
    Ok(())
}

fn record_choice(
    store: &mut ReviewStore,
    row: &observed::Row,
    family: Family,
    reason: &str,
) -> Result<u32, String> {
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
        Family::Spacing => spacing_rules::record_dismissal(
            &mut store.spacing_entries,
            &row.checked_spacing()?,
            reason.into(),
        ),
    };
    result.map_err(|error| format!("{error:?}"))?;
    Ok(match family {
        Family::Parameter => 3,
        Family::Length => 4,
        Family::Spacing => 5,
    })
}

impl Family {
    fn name(self) -> &'static str {
        match self {
            Self::Parameter => "Parameter",
            Self::Length => "Length",
            Self::Spacing => "Spacing",
        }
    }

    fn visible(self, row: &observed::Row) -> bool {
        match self {
            Self::Parameter => row.parameters.visible,
            Self::Length => row.lengths.visible,
            Self::Spacing => row.spacing_requested && row.spacings.visible,
        }
    }

    fn assessment_complete(self, row: &observed::Row) -> bool {
        match self {
            Self::Parameter => row.parameters.complete,
            Self::Length => row.lengths.complete,
            Self::Spacing => row.spacing_requested && row.spacings.complete,
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
            Self::Spacing => summary::Contract {
                schema: "rstyle.spacing-review/1",
                incomplete_scope: "Spacing source assessments or positive finding identities remain unavailable",
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
            Self::Spacing => inputs.store.spacing_entries.len(),
        };
        summarize(
            decisions,
            retained,
            summary::Scope {
                requested: inputs.requested && self.requested(inputs.observations),
                complete: self.complete(inputs.observations),
            },
            self,
        )
    }

    fn requested(self, observations: &observed::Observations) -> bool {
        !matches!(self, Self::Spacing) || observations.spacing_requested
    }

    fn not_requested(self) -> Report {
        summarize(
            Vec::new(),
            0,
            summary::Scope {
                requested: false,
                complete: false,
            },
            self,
        )
    }
}

impl Reports {
    fn not_requested() -> Self {
        Self {
            baseline: None,
            parameters: Family::Parameter.not_requested(),
            lengths: Family::Length.not_requested(),
            spacings: Family::Spacing.not_requested(),
        }
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
