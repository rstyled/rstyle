//! Advisory edit coordination from chosen helper wrapper intents.
mod model;
mod overlap;
pub(super) mod parameters;
#[cfg(test)]
mod tests;

use super::{Function, analysis, report};
use analysis::{Coverage, Execution, FactAvailability};
use rstyle_pdg::consumer_facts::{Availability, planning::BodyRegion};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Component, Path},
};

#[derive(Serialize)]
pub(super) struct Report {
    schema: &'static str,
    intent: &'static str,
    execution: Execution,
    coverage: Coverage,
    expected_plans: Option<usize>,
    plans: Vec<Plan>,
    regions: Vec<Region>,
    // Filled from the bounded pure overlap consumer.
    relations: Vec<model::Pair>,
    overlap_groups: Vec<Vec<usize>>,
    expected_comparisons: Option<usize>,
    assessed_comparisons: usize,
    region_comparisons: usize,
    snapshot_comparison_bytes: usize,
    conflicting_snapshot_plans: Vec<usize>,
    unavailable_plans: Vec<usize>,
    whole_plan_independence: &'static str,
    stack_order: &'static str,
    unknown_edits: &'static [&'static str],
    reasons: Vec<String>,
}

#[derive(Serialize)]
struct Plan {
    id: usize,
    members: [String; 2],
    required_regions: Vec<usize>,
    unavailable_regions: Vec<Unavailable>,
}

#[derive(Clone, Serialize)]
struct Unavailable {
    function: String,
    availability: FactAvailability,
    execution: Execution,
    reason: String,
}

#[derive(Serialize)]
struct Region {
    function: String,
    resource: String,
    package_manifest: String,
    span: rstyle_pdg::ByteSpan,
    #[serde(skip)]
    original: BodyRegion,
}

#[derive(Default)]
struct Contents {
    plans: Vec<Plan>,
    regions: Vec<Region>,
}

#[derive(Default)]
struct Population {
    requested: bool,
    expected_plans: Option<usize>,
    scope_complete: bool,
    over_budget: bool,
}

struct Resolver<'a> {
    records: BTreeMap<String, &'a Function>,
    resolved: BTreeMap<String, Result<usize, Unavailable>>,
    regions: Vec<Region>,
}

pub(super) fn evaluate(
    helpers: &analysis::helper_reports::Report<'_>,
    ctx: &report::Context<'_>,
) -> Result<Report, String> {
    if !ctx.args.edit_plans {
        return Ok(Report::new(Contents::default(), Population::default()));
    }

    let findings: Vec<_> = helpers.finding_pairs().collect();
    let over_budget = findings.len() > model::MAX_PLANS;
    let population = Population {
        requested: true,
        expected_plans: helpers.scope_complete().then_some(findings.len()),
        scope_complete: helpers.scope_complete() && !over_budget,
        over_budget,
    };
    let mut resolver = Resolver::new(ctx);
    let plans = findings
        .into_iter()
        .take(model::MAX_PLANS)
        .enumerate()
        .map(|(id, members)| resolver.plan(id, members, ctx))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Report::new(
        Contents {
            plans,
            regions: resolver.regions,
        },
        population,
    ))
}

impl<'a> Resolver<'a> {
    fn new(ctx: &'a report::Context<'_>) -> Self {
        Self {
            records: ctx
                .handoff
                .assessed
                .iter()
                .map(|record| (record.key.to_string(), record))
                .collect(),
            resolved: BTreeMap::new(),
            regions: Vec::new(),
        }
    }

    fn plan(
        &mut self,
        id: usize,
        members: (&str, &str),
        ctx: &report::Context<'_>,
    ) -> Result<Plan, String> {
        let endpoints = [self.resolve(members.0, ctx)?, self.resolve(members.1, ctx)?];
        Ok(Plan {
            id,
            members: [members.0.into(), members.1.into()],
            required_regions: endpoints
                .iter()
                .filter_map(|result| result.as_ref().ok().copied())
                .collect(),
            unavailable_regions: endpoints.into_iter().filter_map(Result::err).collect(),
        })
    }

    fn resolve(
        &mut self,
        endpoint: &str,
        ctx: &report::Context<'_>,
    ) -> Result<Result<usize, Unavailable>, String> {
        if let Some(result) = self.resolved.get(endpoint) {
            return Ok(result.clone());
        }

        let record = self
            .records
            .get(endpoint)
            .ok_or("Missing planned helper endpoint")?;
        let result = region(record, ctx)?.map(|region| {
            let index = self.regions.len();
            self.regions.push(region);
            index
        });
        self.resolved.insert(endpoint.into(), result.clone());
        Ok(result)
    }
}

fn region(
    record: &Function,
    ctx: &report::Context<'_>,
) -> Result<Result<Region, Unavailable>, String> {
    let original = match original_region(record)? {
        Ok(original) => original,
        Err(failure) => return Ok(Err(failure)),
    };
    located_region(record, ctx, original)
}

fn located_region(
    record: &Function,
    ctx: &report::Context<'_>,
    original: BodyRegion,
) -> Result<Result<Region, Unavailable>, String> {
    let manifest = package_manifest(record, ctx)?;
    let (Some(resource), Some(package_manifest)) = (
        relative(original.path(), &ctx.capture.plan.root),
        relative(manifest, &ctx.capture.plan.root),
    ) else {
        return Ok(Err(unavailable(
            record,
            FactAvailability::Unknown,
            "Planning source or manifest has no unambiguous workspace-relative locator".into(),
        )));
    };
    Ok(Ok(Region {
        function: record.key.to_string(),
        resource,
        package_manifest,
        span: original.span,
        original,
    }))
}

fn original_region(record: &Function) -> Result<Result<BodyRegion, Unavailable>, String> {
    Ok(
        match rstyle_pdg::consumer_facts::planning::inspect(&record.facts)? {
            Availability::Known(region) => Ok(region),
            Availability::Unknown { reason } => {
                Err(unavailable(record, FactAvailability::Unknown, reason))
            }
            Availability::Unsupported { reason } => {
                Err(unavailable(record, FactAvailability::Unsupported, reason))
            }
            Availability::NotApplicable { reason } => {
                Err(unavailable(record, FactAvailability::NotApplicable, reason))
            }
        },
    )
}

fn package_manifest<'a>(
    record: &Function,
    ctx: &'a report::Context<'_>,
) -> Result<&'a Path, String> {
    let context = ctx
        .handoff
        .unit_contexts
        .iter()
        .find(|context| context.id == record.key.unit())
        .ok_or("Missing planning unit context")?;
    let unit = ctx
        .capture
        .units
        .get(context.observed_unit)
        .ok_or("Invalid planning unit context")?;
    let package = unit
        .target
        .package_context
        .as_ref()
        .ok_or("Missing planning package context")?;

    if package.package_id != unit.target.package
        || package.manifest_path.parent() != Some(unit.target.manifest_dir.as_path())
    {
        return Err("Planning package association is contradictory".into());
    }
    Ok(&package.manifest_path)
}

fn unavailable(record: &Function, availability: FactAvailability, reason: String) -> Unavailable {
    let execution = if reason == rstyle_pdg::consumer_facts::planning::SYNTAX_BUDGET_REASON {
        Execution::BudgetExhausted
    } else {
        Execution::Completed
    };
    Unavailable {
        function: record.key.to_string(),
        availability,
        execution,
        reason,
    }
}

fn relative(path: &Path, root: &Path) -> Option<String> {
    let components = path
        .strip_prefix(root)
        .ok()?
        .components()
        .map(|part| match part {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    (!components.is_empty()).then(|| components.join("/"))
}

impl Contents {
    fn analyze(&self, complete: bool) -> model::Analysis {
        let plans = self
            .plans
            .iter()
            .map(|plan| model::Plan {
                required_regions: 2,
                id: plan.id,
                regions: plan
                    .required_regions
                    .iter()
                    .map(|index| {
                        let region = &self.regions[*index];
                        model::Region {
                            resource: &region.resource,
                            snapshot: region.original.snapshot(),
                            span: region.span,
                        }
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();
        overlap::analyze(&plans, complete)
    }

    fn exhausted(&self) -> bool {
        self.plans
            .iter()
            .flat_map(|plan| &plan.unavailable_regions)
            .any(|failure| failure.execution == Execution::BudgetExhausted)
    }
}

struct Summary {
    execution: Execution,
    coverage: Coverage,
    reasons: Vec<String>,
}

impl Summary {
    fn new(contents: &Contents, population: &Population, result: &model::Analysis) -> Self {
        let exhausted = population.over_budget
            || result.execution == model::Execution::BudgetExhausted
            || (population.scope_complete
                && population.expected_plans != Some(contents.plans.len()))
            || contents.exhausted();

        let execution = if !population.requested {
            Execution::NotRequested
        } else if exhausted {
            Execution::BudgetExhausted
        } else {
            Execution::Completed
        };
        let coverage = if result.coverage_complete {
            Coverage::Complete
        } else {
            Coverage::Incomplete
        };
        let reasons = [
            (!population.scope_complete && population.requested).then_some(
                "Potential helper findings remain unavailable; see helper_proposals coverage"),
            (coverage == Coverage::Incomplete && population.requested).then_some(
                "Required edit coordination is incomplete; inspect plan, relation and execution outcomes"),
        ].into_iter().flatten().map(str::to_owned).collect();
        Self {
            execution,
            coverage,
            reasons,
        }
    }
}

impl Report {
    fn new(contents: Contents, population: Population) -> Self {
        let complete =
            population.scope_complete && population.expected_plans == Some(contents.plans.len());
        let result = contents.analyze(complete);
        let summary = Summary::new(&contents, &population, &result);
        Self {
            schema: "rstyle.edit-planning/1",
            intent: "shared_helper_with_original_signature_wrappers",
            execution: summary.execution,
            coverage: summary.coverage,
            expected_plans: population.expected_plans,
            plans: contents.plans,
            regions: contents.regions,
            relations: result.relations,
            overlap_groups: result.overlap_groups,
            expected_comparisons: result.expected_comparisons,
            assessed_comparisons: result.assessed_comparisons,
            region_comparisons: result.region_comparisons,
            snapshot_comparison_bytes: result.snapshot_comparison_bytes,
            conflicting_snapshot_plans: result.conflicting_snapshot_plans,
            unavailable_plans: result.unavailable_plans,
            whole_plan_independence: "unknown",
            stack_order: "not_proven",
            unknown_edits: &["helper_declaration_insertion", "helper_name_and_scope"],
            reasons: summary.reasons,
        }
    }

    pub(super) fn permits_success(&self) -> bool {
        self.execution == Execution::NotRequested
            || (self.execution == Execution::Completed && self.coverage == Coverage::Complete)
    }

    pub(super) fn render(&self) -> String {
        if self.execution == Execution::NotRequested {
            return String::new();
        }
        format!(
            "\nEdit planning: {:?} coverage; {} helper wrapper intents; {} overlap review groups. Helper insertion/name/scope remain unknown; no safe rewrite or stack order is proved.",
            self.coverage,
            self.plans.len(),
            self.overlap_groups.len()
        )
    }
}
