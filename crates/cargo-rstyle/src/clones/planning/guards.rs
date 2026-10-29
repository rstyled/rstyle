//! Required original tail-if regions for advisory guard-clause review.
use super::{Region, Unavailable, located_region, model, unavailable};
use crate::clones::{Function, FunctionKey, analysis, report};
use analysis::{Completion, Coverage, Execution, FactAvailability};
use rstyle_pdg::{
    ByteSpan,
    consumer_facts::{Availability, planning},
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub(in crate::clones) struct Report {
    schema: &'static str,
    intent: &'static str,
    execution: Execution,
    coverage: Coverage,
    findings: usize,
    expected_plans: Option<usize>,
    plans: Vec<Plan>,
    whole_plan_independence: &'static str,
    stack_order: &'static str,
    unknown_edits: &'static [&'static str],
    reasons: Vec<String>,
}

#[derive(Serialize)]
struct Plan {
    id: usize,
    function: String,
    region: Option<Region>,
    #[serde(skip_serializing_if = "Option::is_none")]
    closure_span: Option<ByteSpan>,
    unavailable_regions: Vec<Unavailable>,
}

pub(in crate::clones) fn evaluate(
    ctx: &report::Context<'_>,
    analysis: &analysis::Report<'_>,
) -> Result<Option<Report>, String> {
    if !ctx.args.guard_plans {
        return Ok(None);
    }
    let records: BTreeMap<_, _> = ctx
        .handoff
        .assessed
        .iter()
        .map(|record| (record.key, record))
        .collect();
    let plans = analysis
        .guard_findings()
        .take(model::MAX_PLANS)
        .enumerate()
        .map(|(id, (key, span))| plan(ctx, &records, id, (key, span)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(Report::new(analysis, plans)))
}

fn plan(
    ctx: &report::Context<'_>,
    records: &BTreeMap<FunctionKey, &Function>,
    id: usize,
    finding: (FunctionKey, &analysis::guards::GuardOpportunity),
) -> Result<Plan, String> {
    let record = records
        .get(&finding.0)
        .ok_or("Missing planned guard function")?;
    let region = match planning::inspect_subregion(record.facts.full()?, finding.1.span)? {
        Availability::Known(original) => located_region(record, ctx, original)?,
        Availability::Unknown { reason } => {
            Err(unavailable(record, FactAvailability::Unknown, reason))
        }
        Availability::Unsupported { reason } => {
            Err(unavailable(record, FactAvailability::Unsupported, reason))
        }
        Availability::NotApplicable { reason } => {
            Err(unavailable(record, FactAvailability::NotApplicable, reason))
        }
    };
    let (region, unavailable_regions) = match region {
        Ok(region) => (Some(region), vec![]),
        Err(reason) => (None, vec![reason]),
    };
    Ok(Plan {
        id,
        function: finding.0.to_string(),
        region,
        closure_span: finding.1.closure_span,
        unavailable_regions,
    })
}

impl Report {
    fn new(analysis: &analysis::Report<'_>, plans: Vec<Plan>) -> Self {
        let findings = analysis.guard_findings().count();
        let population_complete = analysis.guard_population_complete();
        let execution =
            if analysis.guard_execution() == Execution::Completed && findings > plans.len() {
                Execution::BudgetExhausted
            } else {
                analysis.guard_execution()
            };
        let complete = population_complete
            && execution == Execution::Completed
            && plans.iter().all(|plan| plan.region.is_some());

        Self {
            schema: "rstyle.guard-planning/1",
            intent: "early_scalar_return_and_lift_continuing_branch",
            execution,
            coverage: if complete {
                Coverage::Complete
            } else {
                Coverage::Incomplete
            },
            findings,
            expected_plans: population_complete.then_some(findings),
            plans,
            whole_plan_independence: "unknown",
            stack_order: "not_proven",
            unknown_edits: &[
                "condition_inversion_and_replacement_design",
                "lifetimes_and_drops",
                "formatting_and_comments",
            ],
            reasons: (!complete)
                .then_some("Guard intent population or original regions are incomplete".into())
                .into_iter()
                .collect(),
        }
    }

    pub(super) fn execution(&self) -> Execution {
        self.execution
    }

    pub(super) fn complete(&self) -> bool {
        self.permits_success()
    }

    pub(super) fn footprints(&self) -> impl Iterator<Item = model::Plan<'_>> {
        self.plans.iter().map(|plan| model::Plan {
            id: plan.id,
            required_regions: 1,
            regions: plan.region.iter().map(Region::footprint).collect(),
        })
    }

    pub(in crate::clones) fn render(&self) -> String {
        format!(
            "\nGuard planning: {:?} coverage; {} tail-if intents. Replacement design, lifetimes and drops remain unproved.",
            self.coverage,
            self.plans.len()
        )
    }
}

impl Completion for Report {
    fn permits_success(&self) -> bool {
        self.execution == Execution::Completed && self.coverage == Coverage::Complete
    }
}
