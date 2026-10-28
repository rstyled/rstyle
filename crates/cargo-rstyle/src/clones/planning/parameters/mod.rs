//! Signature intents and exact witnessed callers, independent of body effects.
mod collect;
mod regions;

use super::{Region, model, overlap};
use crate::clones::{FunctionKey, analysis, report};
use analysis::{Completion, Coverage, Execution};
use rstyle_pdg::{
    ByteSpan,
    consumer_facts::{Availability, callers},
};
use serde::Serialize;

#[derive(Serialize)]
pub(in crate::clones) struct Report {
    schema: &'static str,
    execution: Execution,
    coverage: Coverage,
    findings: usize,
    expected_plans: Option<usize>,
    caller_scope: &'static str,
    call_resolution: &'static str,
    plans: Vec<Plan>,
    caller_census: Vec<UnitCensus>,
    coordination: model::Analysis,
    whole_plan_independence: &'static str,
    stack_order: &'static str,
    unknown_edits: &'static [&'static str],
    reasons: Vec<String>,
}

#[derive(Serialize)]
struct Plan {
    id: usize,
    function: String,
    signature: Option<Region>,
    callers: Vec<Caller>,
    reasons: Vec<String>,
}

#[derive(Serialize)]
struct Caller {
    function: String,
    region: Region,
    arguments: Vec<Argument>,
}

#[derive(Serialize)]
struct Argument {
    span: ByteSpan,
    formal: Availability<Option<u32>>,
}

#[derive(Serialize)]
struct UnitCensus {
    unit: usize,
    #[serde(flatten)]
    census: callers::Census,
}

pub(in crate::clones) fn evaluate(
    ctx: &report::Context<'_>,
    analysis: &analysis::Report<'_>,
) -> Result<Option<Report>, String> {
    if !ctx.args.parameter_plans {
        return Ok(None);
    }

    let keys: Vec<_> = analysis.parameter_findings().collect();
    let complete = analysis.parameter_population_complete();
    let mut collected = collect::evaluate(ctx, &keys)?;
    collected.budget |= analysis.execution == Execution::BudgetExhausted;
    let mut report = Report::new(collected, keys.len(), complete);
    if analysis.execution == Execution::Failed {
        report.execution = Execution::Failed;
    }
    Ok(Some(report))
}

pub(super) struct Collected {
    plans: Vec<Plan>,
    census: Vec<UnitCensus>,
    budget: bool,
}

impl Collected {
    fn analyze(&self, population_complete: bool) -> model::Analysis {
        let complete = population_complete
            && !self.budget
            && self.plans.iter().all(|plan| plan.reasons.is_empty())
            && self.census.iter().all(|unit| {
                unit.census.coverage == rstyle_pdg::consumer_facts::mutations::Coverage::Complete
            });
        let plans = self.plans.iter().map(Plan::footprint).collect::<Vec<_>>();
        overlap::analyze(&plans, complete)
    }
}

impl Report {
    fn new(collected: Collected, findings: usize, population_complete: bool) -> Self {
        let coordination =
            collected.analyze(population_complete && findings == collected.plans.len());
        let budget =
            collected.budget || coordination.execution == model::Execution::BudgetExhausted;
        let coverage = if coordination.coverage_complete {
            Coverage::Complete
        } else {
            Coverage::Incomplete
        };
        Self {
            schema: "rstyle.parameter-planning/1",
            execution: if budget {
                Execution::BudgetExhausted
            } else {
                Execution::Completed
            },
            coverage,
            findings,
            expected_plans: population_complete.then_some(findings),
            caller_scope: "captured_selected_units",
            call_resolution: "same_unit_as_target",
            plans: collected.plans,
            caller_census: collected.census,
            coordination,
            whole_plan_independence: "unknown",
            stack_order: "not_proven",
            unknown_edits: &[
                "replacement_signature_and_argument_design",
                "external_or_uncaptured_callers",
                "function_pointer_and_trait_uses",
                "imports_names_and_visibility",
            ],
            reasons: (!population_complete)
                .then_some("Parameter finding population is incomplete")
                .into_iter()
                .map(str::to_owned)
                .collect(),
        }
    }

    pub(in crate::clones) fn render(&self) -> String {
        format!(
            "\nParameter planning: {:?} coverage; {} signature intents. Caller coverage is limited to captured selected units; whole-plan independence and stack order remain unknown.",
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

impl Plan {
    fn new(id: usize, function: FunctionKey, signature: Result<Region, String>) -> Self {
        let (signature, reasons) = match signature {
            Ok(region) => (Some(region), vec![]),
            Err(reason) => (None, vec![reason]),
        };
        Self {
            id,
            function: function.to_string(),
            signature,
            callers: vec![],
            reasons,
        }
    }

    fn footprint(&self) -> model::Plan<'_> {
        let regions = self
            .signature
            .iter()
            .chain(self.callers.iter().map(|caller| &caller.region))
            .map(|region| model::Region {
                resource: &region.resource,
                snapshot: region.original.snapshot(),
                span: region.span,
            })
            .collect();
        model::Plan {
            id: self.id,
            required_regions: 1 + self.callers.len(),
            regions,
        }
    }
}
