//! Compiler style choices reuse exact numeric review accounting and completed owner scopes.
pub(in crate::clones) mod demand;
pub(in crate::clones) mod observed;
pub(in crate::clones) mod retained;
mod rules;
use super::{
    ReviewStore, exact_rules,
    numeric_workflow::{self, Visibility},
    outcome, positive_workflow, status, summary, workflow,
};
use crate::clones::{analysis::Completion, identity::UnitId, report};
pub(in crate::clones) use rules::Snapshot;
pub(super) use rules::{Entry, validate_entries};
use serde::Serialize;

pub(in crate::clones) struct Row<'a> {
    pub unit: UnitId,
    pub index: usize,
    pub path: &'a str,
    pub snapshot: &'a Result<Snapshot, String>,
}

impl positive_workflow::Row for Row<'_> {
    type Snapshot = Snapshot;
    fn snapshot(&self) -> &Result<Snapshot, String> {
        self.snapshot
    }
}

#[derive(Serialize)]
pub(in crate::clones) struct Report {
    #[serde(flatten)]
    review: numeric_workflow::Presentation<Decision>,
}

#[derive(Serialize)]
struct Decision {
    unit: UnitId,
    diagnostic_index: usize,
    #[serde(flatten)]
    outcome: outcome::Outcome,
}

impl summary::Decision for Decision {
    fn state(&self) -> &'static str {
        self.outcome.state
    }
}
impl Visibility for Decision {
    type Key = (UnitId, usize);
    const CONTRACT: summary::Contract = summary::Contract {
        schema: "rstyle.compiler-diagnostic-review/1",
        incomplete_scope: "Compiler diagnostic review inputs are incomplete",
    };
    fn hidden_key(&self) -> Option<Self::Key> {
        (self.outcome.state == "hidden_exact").then_some((self.unit, self.diagnostic_index))
    }
}

pub(super) fn apply(
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
    store: Option<&mut ReviewStore>,
) -> Result<(Report, Vec<status::Record>), String> {
    let Some(store) = store else {
        return Ok((summarize(Vec::new(), 0, false, true), Vec::new()));
    };
    let rows = inputs
        .compiler_advisory
        .map(|report| report.diagnostic_reviews())
        .unwrap_or_default();
    if !ctx.args.dismiss_compiler_diagnostic.is_empty() {
        dismiss(store, &rows, ctx)?;
    }

    let evaluated = positive_workflow::evaluate(
        &store.compiler_diagnostic_entries,
        &rows,
        positive_workflow::Contract {
            retain_records: inputs.compiler_advisory.is_none(),
            requested: ctx.args.compiler_advisory,
            family: status::Family::CompilerDiagnostic,
            label: "Compiler diagnostic",
        },
    )?;
    let decisions = rows
        .iter()
        .zip(evaluated.outcomes)
        .map(|(row, outcome)| Decision {
            unit: row.unit,
            diagnostic_index: row.index,
            outcome,
        })
        .collect();
    let records = retained_records(inputs, ctx, store, evaluated.records);
    Ok((
        summarize(
            decisions,
            store.compiler_diagnostic_entries.len(),
            ctx.args.compiler_advisory,
            inputs
                .compiler_advisory
                .is_some_and(|report| report.advisory_inputs_complete()),
        ),
        records,
    ))
}

fn retained_records(
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
    store: &ReviewStore,
    fallback: Vec<status::Record>,
) -> Vec<status::Record> {
    if !ctx.args.review_status_requested() {
        return Vec::new();
    }
    inputs.compiler_advisory.map_or(fallback, |advisory| {
        let observations = retained::observations(advisory, ctx);
        numeric_workflow::records(&store.compiler_diagnostic_entries, &observations, ctx)
    })
}

fn dismiss(
    store: &mut ReviewStore,
    rows: &[Row<'_>],
    ctx: &report::Context<'_>,
) -> Result<(), String> {
    let [unit, index] = ctx.args.dismiss_compiler_diagnostic.as_slice() else {
        return Err(
            "A compiler diagnostic dismissal requires a saved-run unit and diagnostic index".into(),
        );
    };
    let unit: u32 = unit
        .parse()
        .map_err(|_| "Compiler diagnostic unit must be an unsigned integer")?;
    let index: usize = index
        .parse()
        .map_err(|_| "Compiler diagnostic index must be an unsigned integer")?;
    let row = rows
        .iter()
        .find(|row| row.unit.ordinal() == unit && row.index == index)
        .ok_or("Selected saved-run compiler diagnostic is not a current finding")?;

    let snapshot = row.snapshot.as_ref().map_err(Clone::clone)?;
    let current =
        exact_rules::LiveFinding::checked(snapshot).map_err(|error| format!("{error:?}"))?;
    exact_rules::record_with_reason(
        &mut store.compiler_diagnostic_entries,
        &current,
        ctx.args.review_reason.as_deref(),
    )?;
    store.schema_version = store.schema_version.max(9);
    Ok(())
}

fn summarize(decisions: Vec<Decision>, retained: usize, requested: bool, complete: bool) -> Report {
    Report {
        review: numeric_workflow::Presentation::new(
            decisions,
            retained,
            summary::Scope {
                requested,
                complete,
            },
        ),
    }
}

impl Report {
    pub(in crate::clones) fn requested(&self) -> bool {
        self.review.summary.requested()
    }
    pub(in crate::clones) fn hidden_for(&self, unit: UnitId, index: usize) -> bool {
        self.review.hidden.contains(&(unit, index))
    }
    pub(in crate::clones) fn render(&self) -> String {
        self.review.summary.render("Compiler diagnostic")
    }
}
impl Completion for Report {
    fn permits_success(&self) -> bool {
        self.review.summary.permits_success()
    }
}
