//! Guard reviews borrow completed opportunities and existing original-owner identities.
use super::{
    Locator, ReviewStore, exact_rules, guard_rules, local, local_adapter, outcome,
    owner_observations, owner_population, positive_workflow, status, summary, workflow,
};
use crate::clones::{FunctionKey, analysis, report};
use rstyle_pdg::source_evidence::byte_range;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

type Identity = (Locator, local::BuildContext);

#[derive(Serialize)]
pub(in crate::clones) struct Report {
    #[serde(flatten)]
    summary: summary::Summary,
    decisions: Vec<Decision>,
    #[serde(skip)]
    hidden: BTreeSet<(FunctionKey, usize)>,
}

#[derive(Serialize)]
struct Decision {
    key: FunctionKey,
    opportunity_index: usize,
    #[serde(flatten)]
    outcome: outcome::Outcome,
}

struct Owner<'a> {
    review: analysis::GuardReview<'a>,
    identity: Result<Identity, String>,
    unique: bool,
}

struct Row {
    key: FunctionKey,
    opportunity_index: usize,
    selected: bool,
    snapshot: Result<guard_rules::Snapshot, String>,
}

pub(super) fn apply(
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
    store: Option<&mut ReviewStore>,
) -> Result<(Report, Vec<status::Record>), String> {
    let requested = ctx.args.guard_clauses;
    let Some(store) = store.filter(|_| requested || ctx.args.review_status_requested()) else {
        return Ok((summarize(Vec::new(), 0, false, false), Vec::new()));
    };
    let (rows, truncated, unavailable_scope) = collect(inputs, ctx);
    dismiss(store, &rows, ctx, truncated)?;
    let evaluated = positive_workflow::evaluate(
        &store.guard_entries,
        &rows,
        positive_workflow::Contract {
            requested,
            retain_records: true,
            family: status::Family::GuardClause,
            label: "Guard clause",
        },
    )?;

    let decisions = rows
        .into_iter()
        .zip(evaluated.outcomes)
        .map(|(row, outcome)| Decision {
            key: row.key,
            opportunity_index: row.opportunity_index,
            outcome,
        })
        .collect();

    let complete = inputs.analysis.guard_population_complete() && !truncated && !unavailable_scope;
    Ok((
        summarize(decisions, store.guard_entries.len(), requested, complete),
        evaluated.records,
    ))
}

fn collect(inputs: &workflow::Inputs<'_, '_>, ctx: &report::Context<'_>) -> (Vec<Row>, bool, bool) {
    let collected = owner_observations::collect(
        inputs
            .analysis
            .guard_reviews()
            .map(|review| (review.key, review)),
        inputs,
        ctx,
        |review, identity| Owner {
            review,
            identity,
            unique: false,
        },
    );

    let selected = selected_owners(ctx);
    let mut rows = Vec::new();
    for owner in &collected.rows {
        append_rows(&mut rows, owner, &selected);
    }

    let truncated = collected.truncated || rows.len() > exact_rules::MAX_ENTRIES;
    rows.truncate(exact_rules::MAX_ENTRIES);
    (rows, truncated, collected.unavailable_scope)
}

fn append_rows(rows: &mut Vec<Row>, owner: &Owner<'_>, selected: &BTreeMap<FunctionKey, bool>) {
    let assessment = owner.review.assessment;
    for (index, opportunity) in assessment
        .opportunity
        .iter()
        .chain(&assessment.closure_opportunities)
        .enumerate()
    {
        if rows.len() > exact_rules::MAX_ENTRIES {
            break;
        }
        let snapshot = snapshot(owner, opportunity, index);
        rows.push(Row {
            key: owner.review.key,
            opportunity_index: index,
            selected: selected.get(&owner.review.key).copied().unwrap_or(false),
            snapshot,
        });
    }
}

fn snapshot(
    owner: &Owner<'_>,
    opportunity: &analysis::guards::GuardOpportunity,
    index: usize,
) -> Result<guard_rules::Snapshot, String> {
    if owner.review.assessment.execution != analysis::Execution::Completed {
        return Err("Guard opportunity assessment did not complete".into());
    }
    if !owner.unique {
        return Err("Guard review owner is ambiguous or truncated".into());
    }

    let (locator, context) = owner.identity.as_ref().map_err(Clone::clone)?;
    let source = owner
        .review
        .source
        .ok_or("Guard review original source is unavailable")?;
    if source.len() > guard_rules::MAX_SOURCE {
        return Err("Guard review original source exceeds its budget".into());
    }

    let closure_source = opportunity
        .closure_span
        .map(|span| byte_range(source, span).map(str::to_owned))
        .transpose()?;
    let region = if closure_source.is_some() {
        guard_rules::Region::Closure {
            ordinal: index - usize::from(owner.review.assessment.opportunity.is_some()),
        }
    } else {
        guard_rules::Region::FunctionTail
    };

    Ok(guard_rules::Snapshot {
        versions: local_adapter::versions("guard_clause"),
        locator: locator.clone(),
        context: context.clone(),
        region,
        guard_source: byte_range(source, opportunity.span)?.to_owned(),
        closure_source,
        fallback_branch: opportunity.fallback_branch,
        continuing_statements: opportunity.continuing_statements,
    })
}

fn selected_owners(ctx: &report::Context<'_>) -> BTreeMap<FunctionKey, bool> {
    ctx.handoff
        .assessed
        .iter()
        .map(|function| {
            let selected = !ctx.args.select_guards
                || ctx
                    .analysis_config
                    .and_then(analysis::Config::changed_files)
                    .is_none_or(|selection| {
                        analysis::selected_file(function, selection, &ctx.capture.plan.root)
                    });
            (function.key, selected)
        })
        .collect()
}

fn dismiss(
    store: &mut ReviewStore,
    rows: &[Row],
    ctx: &report::Context<'_>,
    truncated: bool,
) -> Result<(), String> {
    if ctx.args.dismiss_guard.is_empty() {
        return Ok(());
    }
    if truncated {
        return Err("Guard review identity budget exhausted; no dismissal recorded".into());
    }

    let [key, index] = ctx.args.dismiss_guard.as_slice() else {
        return Err("Guard dismissal requires an owner key and opportunity index".into());
    };
    let index: usize = index
        .parse()
        .map_err(|_| "Guard opportunity index is invalid")?;
    let row = rows
        .iter()
        .find(|row| row.key.to_string() == *key && row.opportunity_index == index && row.selected)
        .ok_or("Selected saved-run guard opportunity is unavailable or outside selection")?;

    let snapshot = row.snapshot.as_ref().map_err(Clone::clone)?;
    let live = exact_rules::LiveFinding::checked(snapshot).map_err(|error| format!("{error:?}"))?;
    exact_rules::record_with_reason(
        &mut store.guard_entries,
        &live,
        ctx.args.review_reason.as_deref(),
    )?;
    store.schema_version = store.schema_version.max(10);
    Ok(())
}

fn summarize(decisions: Vec<Decision>, retained: usize, requested: bool, complete: bool) -> Report {
    let hidden = decisions
        .iter()
        .filter(|decision| decision.outcome.state == "hidden_exact")
        .map(|decision| (decision.key, decision.opportunity_index))
        .collect();
    let summary = summary::summarize(
        &decisions,
        retained,
        summary::Scope {
            requested,
            complete,
        },
        summary::Contract {
            schema: "rstyle.guard-review/1",
            incomplete_scope: "Guard owner population or opportunity identities remain incomplete",
        },
    );
    Report {
        summary,
        decisions,
        hidden,
    }
}

impl owner_population::Owner for Owner<'_> {
    fn identity(&self) -> Result<(&Locator, &local::BuildContext), &str> {
        self.identity
            .as_ref()
            .map(|(locator, context)| (locator, context))
            .map_err(String::as_str)
    }
}

impl owner_observations::Observation for Owner<'_> {
    fn key(&self) -> FunctionKey {
        self.review.key
    }
    fn set_unique(&mut self, unique: bool) {
        self.unique = unique;
    }
}

impl positive_workflow::Row for Row {
    type Snapshot = guard_rules::Snapshot;
    fn snapshot(&self) -> &Result<Self::Snapshot, String> {
        &self.snapshot
    }
}

impl summary::Decision for Decision {
    fn state(&self) -> &'static str {
        self.outcome.state
    }
}

impl Report {
    pub(in crate::clones) fn hidden_for(&self, key: FunctionKey, index: usize) -> bool {
        self.hidden.contains(&(key, index))
    }
    pub(in crate::clones) fn render(&self) -> String {
        let mut output = self.summary.render("Guard clause");
        if !self.summary.requested() {
            return output;
        }
        for decision in &self.decisions {
            output.push_str(&format!(
                "\nGuard review {} {}: {}. {}",
                decision.key,
                decision.opportunity_index,
                decision.outcome.state,
                decision.outcome.reason.as_deref().unwrap_or("")
            ));
        }
        output
    }
}

impl analysis::Completion for Report {
    fn permits_success(&self) -> bool {
        self.summary.permits_success()
    }
}
