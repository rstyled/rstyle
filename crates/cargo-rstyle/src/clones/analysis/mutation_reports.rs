//! Direct scalar mutation budgets with explicit unknown remainder and provenance.
mod decisions;
mod reviews;
pub(in crate::clones) use reviews::{Review, RuleReview};
mod policy;
mod rendering;
mod scope;

use super::{Coverage, Execution, FactAvailability, Outcome};
use crate::clones::{Args, Function, FunctionKey, Handoff, capture};
pub(in crate::clones) use policy::Settings;
use rstyle_pdg::consumer_facts::{Availability, mutations};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct Report<'a> {
    schema: &'static str,
    detector: &'static str,
    #[serde(flatten)]
    scope: scope::Summary,
    config: Option<&'a Settings>,
    pub findings: usize,
    pub visible_findings: usize,
    review_hidden_findings: usize,
    suppressed_findings: usize,
    excluded_findings: usize,
    functions: Vec<FunctionReport<'a>>,
    limitations: &'static [&'static str],
}

#[derive(Serialize)]
struct FunctionReport<'a> {
    #[serde(flatten)]
    location: Location<'a>,
    execution: Execution,
    outcome: Outcome,
    coverage: Coverage,
    availability: FactAvailability,
    evidence: Option<policy::Evidence>,
    rules: Vec<policy::Rule>,
    policy: decisions::Policy,
    density: Density,
    #[serde(skip)]
    review_hidden: [bool; 2],
    reason: Option<String>,
}

#[derive(Serialize)]
struct Location<'a> {
    key: FunctionKey,
    name: &'a str,
    source_file: Option<&'a str>,
    fn_span: rstyle_pdg::ByteSpan,
}

impl<'a> Location<'a> {
    fn new(function: &'a Function) -> Self {
        Self {
            key: function.key,
            name: function.facts.view().name,
            source_file: function
                .facts
                .view()
                .build
                .frontend_options
                .get("source_file")
                .and_then(serde_json::Value::as_str),
            fn_span: function.facts.view().fn_span,
        }
    }
}

#[derive(Serialize)]
struct Density {
    outcome: Outcome,
    reason: &'static str,
}

pub(in crate::clones) fn configuration(args: &Args) -> Result<Option<Settings>, String> {
    policy::configuration(args)
}

pub(in crate::clones) fn evaluate<'a>(
    handoff: &'a Handoff,
    capture: &capture::Capture,
    args: &Args,
    config: Option<&'a Settings>,
) -> Result<Report<'a>, String> {
    let functions = match config {
        None => Vec::new(),
        Some(settings) => evaluate_functions(handoff, args, settings, &capture.plan.root)?,
    };
    let counts = decisions::FindingCounts::observe(functions.iter().map(|f| &f.policy));
    Ok(Report {
        schema: "rstyle.mutations/2",
        detector: "direct-scalar-mutations/1",
        scope: scope::summary(&functions, handoff, capture, config.is_some()),
        config,
        findings: functions
            .iter()
            .flat_map(|f| &f.rules)
            .filter(|rule| rule.outcome == Outcome::Finding)
            .count()
            + functions
                .iter()
                .filter(|f| f.policy.invalid_suppression.is_some())
                .count(),
        visible_findings: counts.visible,
        review_hidden_findings: 0,
        suppressed_findings: counts.suppressed,
        excluded_findings: counts.excluded,
        functions,
        limitations: &[
            "Counts describe original syntactic assignment sites, not dynamic execution frequency.",
            "Ordinary branches and supported primitive loops each contribute their original sites once; pattern conditions and arbitrary iterators remain incomplete.",
            "Direct u32 owned roots and mutable formal pointees only; unknown boundaries prevent clean conclusions.",
            "Numeric saved reviews affect presentation only; no purity, density, transitive effects or rewrite safety claim.",
        ],
    })
}

fn evaluate_functions<'a>(
    handoff: &'a Handoff,
    args: &Args,
    settings: &Settings,
    root: &std::path::Path,
) -> Result<Vec<FunctionReport<'a>>, String> {
    let limit = args.max_analysis_functions.unwrap_or(usize::MAX);
    handoff
        .assessed
        .iter()
        .enumerate()
        .map(|(ordinal, function)| {
            if ordinal >= limit {
                Ok(unavailable(
                    function,
                    Execution::BudgetExhausted,
                    "Analysis function budget exhausted",
                    settings,
                ))
            } else {
                assess(
                    function,
                    mutations::inspect_with_policy(function.facts.full()?)?,
                    settings,
                    root,
                )
            }
        })
        .collect()
}

fn assess<'a>(
    function: &'a Function,
    observed: mutations::PolicyObservation<'_>,
    settings: &Settings,
    root: &std::path::Path,
) -> Result<FunctionReport<'a>, String> {
    let assessment = observed.direct;
    let (availability, reason) = availability(&assessment.availability);
    let complete = assessment.coverage == mutations::Coverage::Complete
        && assessment.execution == mutations::Execution::Completed
        && availability == FactAvailability::Known;
    let execution = if assessment.execution == mutations::Execution::BudgetExhausted {
        Execution::BudgetExhausted
    } else {
        Execution::Completed
    };

    let evidence = policy::Evidence::from_assessment(assessment, complete)?;
    let rules = settings.evaluate(Some(&evidence), complete);
    let policy = decisions::evaluate(
        super::source_exclusions::evaluate_source(
            &observed.source,
            function,
            root,
            settings.exclusions(),
        ),
        super::source_suppressions::evaluate(&observed.preceding_comment),
        &rules,
        decisions::RawStatus {
            complete,
            execution,
        },
    );
    Ok(FunctionReport {
        execution,
        coverage: if complete {
            Coverage::Complete
        } else {
            Coverage::Incomplete
        },
        availability,
        outcome: policy::outcome(&rules),
        evidence: Some(evidence),
        policy,
        rules,
        reason,
        location: Location::new(function),
        density: Density::unavailable(),
        review_hidden: [false; 2],
    })
}

fn availability(observed: &Availability<()>) -> (FactAvailability, Option<String>) {
    match observed {
        Availability::Known(()) => (FactAvailability::Known, None),
        Availability::Unknown { reason } => (FactAvailability::Unknown, Some(reason.clone())),
        Availability::Unsupported { reason } => {
            (FactAvailability::Unsupported, Some(reason.clone()))
        }
        Availability::NotApplicable { reason } => {
            (FactAvailability::NotApplicable, Some(reason.clone()))
        }
    }
}

fn unavailable<'a>(
    function: &'a Function,
    execution: Execution,
    reason: &str,
    settings: &Settings,
) -> FunctionReport<'a> {
    let rules = settings.evaluate(None, false);
    let policy = decisions::evaluate(
        super::source_exclusions::Report::Unavailable {
            reason: reason.into(),
        },
        super::source_suppressions::Report::failure(reason.into()),
        &rules,
        decisions::RawStatus {
            complete: false,
            execution,
        },
    );
    FunctionReport {
        location: Location::new(function),
        execution,
        outcome: Outcome::NotEvaluated,
        coverage: Coverage::Incomplete,
        availability: FactAvailability::Unknown,
        evidence: None,
        rules,
        policy,
        density: Density::unavailable(),
        review_hidden: [false; 2],
        reason: Some(reason.into()),
    }
}

impl Density {
    fn unavailable() -> Self {
        Self {
            outcome: Outcome::NotEvaluated,
            reason: "Density requires a complete effect vocabulary and statement denominator",
        }
    }
}

impl Report<'_> {
    pub(in crate::clones) fn permits_success(&self) -> bool {
        self.scope.execution == Execution::NotRequested
            || (self.scope.execution == Execution::Completed
                && self.scope.coverage == Coverage::Complete)
    }
}
