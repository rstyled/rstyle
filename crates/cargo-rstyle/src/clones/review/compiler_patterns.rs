//! Exact advisory reviews reuse prepared compiler observations, never clone proof.
pub(in crate::clones) mod observed;
mod rules;
use super::{ReviewStore, outcome, positive_workflow, status, summary, workflow};
use crate::clones::{identity::UnitId, report};
pub(super) use rules::{Entry, validate_entries};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Serialize)]
pub(in crate::clones) struct Report {
    #[serde(flatten)]
    summary: summary::Summary,
    decisions: Vec<Decision>,
    #[serde(skip)]
    hidden: BTreeSet<(UnitId, usize)>,
}

#[derive(Serialize)]
struct Decision {
    unit: UnitId,
    candidate_index: usize,
    candidate_id: String,
    #[serde(flatten)]
    outcome: outcome::Outcome,
}

impl summary::Decision for Decision {
    fn state(&self) -> &'static str {
        self.outcome.state
    }
}

pub(super) fn apply(
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
    store: Option<&mut ReviewStore>,
) -> Result<(Report, Vec<status::Record>), String> {
    let requested = ctx.args.compiler_patterns;
    let Some(store) = store else {
        return Ok((summarize(Vec::new(), 0, false, true), Vec::new()));
    };
    let rows = inputs
        .compiler_advisory
        .map(|report| report.pattern_reviews())
        .unwrap_or_default();
    if !ctx.args.dismiss_compiler_pattern.is_empty() {
        dismiss(store, &rows, ctx)?;
    }

    let evaluated = positive_workflow::evaluate(
        &store.compiler_pattern_entries,
        &rows,
        positive_workflow::Contract {
            requested,
            family: status::Family::CompilerPattern,
            label: "Compiler proposal",
        },
    )?;

    let decisions = rows
        .iter()
        .zip(evaluated.outcomes)
        .map(|(row, outcome)| Decision {
            unit: row.unit,
            candidate_index: row.index,
            candidate_id: row.id.into(),
            outcome,
        })
        .collect();

    let complete = inputs
        .compiler_advisory
        .is_some_and(|report| report.pattern_inputs_complete());
    Ok((
        summarize(
            decisions,
            store.compiler_pattern_entries.len(),
            requested,
            complete,
        ),
        evaluated.records,
    ))
}

fn dismiss(
    store: &mut ReviewStore,
    rows: &[observed::Row<'_>],
    ctx: &report::Context<'_>,
) -> Result<(), String> {
    let row = selected(rows, &ctx.args.dismiss_compiler_pattern)?;
    super::exact_rules::record_with_reason(
        &mut store.compiler_pattern_entries,
        &row.checked()?,
        ctx.args.review_reason.as_deref(),
    )?;
    store.schema_version = store.schema_version.max(8);
    Ok(())
}

fn selected<'a>(
    rows: &'a [observed::Row<'_>],
    selector: &[String],
) -> Result<&'a observed::Row<'a>, String> {
    let [unit, id] = selector else {
        return Err(
            "A compiler proposal dismissal requires a saved-run unit and candidate ID".into(),
        );
    };
    let unit: u32 = unit
        .parse()
        .map_err(|_| "Compiler proposal unit must be an unsigned integer")?;

    let mut selected = rows
        .iter()
        .filter(|row| row.unit.ordinal() == unit && row.id == id);
    let row = selected
        .next()
        .ok_or("Selected saved-run compiler proposal is not a current finding")?;
    if selected.next().is_some() {
        return Err("Compiler candidate ID is ambiguous in the selected unit".into());
    }

    Ok(row)
}

fn summarize(decisions: Vec<Decision>, retained: usize, requested: bool, complete: bool) -> Report {
    let hidden = decisions
        .iter()
        .filter(|row| row.outcome.state == "hidden_exact")
        .map(|row| (row.unit, row.candidate_index))
        .collect();
    let summary = summary::summarize(
        &decisions,
        retained,
        summary::Scope {
            requested,
            complete,
        },
        summary::Contract {
            schema: "rstyle.compiler-pattern-review/1",
            incomplete_scope: "Compiler proposal review identity is unavailable",
        },
    );
    Report {
        summary,
        decisions,
        hidden,
    }
}

impl Report {
    pub(in crate::clones) fn requested(&self) -> bool {
        self.summary.requested()
    }
    pub(in crate::clones) fn hidden_for(&self, unit: UnitId, index: usize) -> bool {
        self.hidden.contains(&(unit, index))
    }

    pub(in crate::clones) fn render(&self) -> String {
        let mut output = self.summary.render("Compiler proposal");
        for row in &self.decisions {
            output.push_str(&format!(
                "\nCompiler proposal unit {} candidate {}: {}. {}",
                row.unit.ordinal(),
                row.candidate_id,
                row.outcome.state,
                row.outcome.reason.as_deref().unwrap_or("")
            ));
        }
        output
    }
}

impl crate::clones::analysis::Completion for Report {
    fn permits_success(&self) -> bool {
        self.summary.permits_success()
    }
}
