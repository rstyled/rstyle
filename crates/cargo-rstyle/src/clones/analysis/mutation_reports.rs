//! Direct scalar mutation budgets with explicit unknown remainder and provenance.
mod policy;

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
    scope: Summary,
    config: Option<&'a Settings>,
    pub findings: usize,
    functions: Vec<FunctionReport<'a>>,
    limitations: &'static [&'static str],
}

#[derive(Serialize)]
struct Summary {
    execution: Execution,
    coverage: Coverage,
    population: &'static str,
    expected_functions: Option<usize>,
    assessed_functions: usize,
    evaluated_functions: usize,
    unavailable_functions: usize,
    reasons: Vec<String>,
}

#[derive(Serialize)]
struct FunctionReport<'a> {
    key: FunctionKey,
    name: &'a str,
    source_file: Option<&'a str>,
    fn_span: rstyle_pdg::ByteSpan,
    execution: Execution,
    outcome: Outcome,
    coverage: Coverage,
    availability: FactAvailability,
    evidence: Option<policy::Evidence>,
    rules: Vec<policy::Rule>,
    density: Density,
    reason: Option<String>,
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
        Some(settings) => evaluate_functions(handoff, args, settings)?,
    };
    Ok(Report {
        schema: "rstyle.mutations/1",
        detector: "direct-scalar-mutations/1",
        scope: summary(&functions, handoff, capture, config.is_some()),
        config,
        findings: functions
            .iter()
            .flat_map(|f| &f.rules)
            .filter(|rule| rule.outcome == Outcome::Finding)
            .count(),
        functions,
        limitations: &[
            "Counts describe original syntactic assignment sites, not dynamic execution frequency.",
            "Direct u32 owned roots and mutable formal pointees only; unknown boundaries prevent clean conclusions.",
            "No purity, density, transitive effects or rewrite safety claim; legacy reviews and suppressions are not applied.",
        ],
    })
}

fn evaluate_functions<'a>(
    handoff: &'a Handoff,
    args: &Args,
    settings: &Settings,
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
                assess(function, mutations::inspect(&function.facts)?, settings)
            }
        })
        .collect()
}

fn assess<'a>(
    function: &'a Function,
    assessment: mutations::Assessment,
    settings: &Settings,
) -> Result<FunctionReport<'a>, String> {
    let (availability, reason) = match &assessment.availability {
        Availability::Known(()) => (FactAvailability::Known, None),
        Availability::Unknown { reason } => (FactAvailability::Unknown, Some(reason.clone())),
        Availability::Unsupported { reason } => {
            (FactAvailability::Unsupported, Some(reason.clone()))
        }
        Availability::NotApplicable { reason } => {
            (FactAvailability::NotApplicable, Some(reason.clone()))
        }
    };
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
        rules,
        reason,
        ..unavailable(function, execution, "", settings)
    })
}

fn unavailable<'a>(
    function: &'a Function,
    execution: Execution,
    reason: &str,
    settings: &Settings,
) -> FunctionReport<'a> {
    FunctionReport {
        key: function.key,
        name: &function.facts.name,
        source_file: function
            .facts
            .build
            .frontend_options
            .get("source_file")
            .and_then(serde_json::Value::as_str),
        fn_span: function.facts.fn_span,
        execution,
        outcome: Outcome::NotEvaluated,
        coverage: Coverage::Incomplete,
        availability: FactAvailability::Unknown,
        evidence: None,
        rules: settings.evaluate(None, false),
        density: Density {
            outcome: Outcome::NotEvaluated,
            reason: "Density requires a complete effect vocabulary and statement denominator",
        },
        reason: Some(reason.into()),
    }
}

fn summary(
    functions: &[FunctionReport<'_>],
    handoff: &Handoff,
    capture: &capture::Capture,
    requested: bool,
) -> Summary {
    let mut reasons: Vec<_> = capture
        .failures
        .iter()
        .chain(&handoff.failures)
        .cloned()
        .collect();
    let expected_functions = reasons.is_empty().then_some(handoff.assessed.len());
    let evaluated_functions = functions
        .iter()
        .filter(|f| f.coverage == Coverage::Complete)
        .count();
    let unavailable_functions = functions.len() - evaluated_functions;
    if requested && unavailable_functions > 0 {
        reasons.push(
            "Some owners have incomplete mutation evidence; see boundaries and reasons".into(),
        );
    }
    let execution = execution(functions, requested, expected_functions.is_some());

    Summary {
        execution,
        coverage: if requested && reasons.is_empty() && execution == Execution::Completed {
            Coverage::Complete
        } else {
            Coverage::Incomplete
        },
        population: "captured function owners, independent of strict graph admission",
        expected_functions,
        assessed_functions: functions
            .iter()
            .filter(|f| f.execution != Execution::BudgetExhausted)
            .count(),
        evaluated_functions,
        unavailable_functions,
        reasons,
    }
}

fn execution(functions: &[FunctionReport<'_>], requested: bool, captured: bool) -> Execution {
    if !requested {
        Execution::NotRequested
    } else if !captured {
        Execution::Failed
    } else if functions
        .iter()
        .any(|f| f.execution == Execution::BudgetExhausted)
    {
        Execution::BudgetExhausted
    } else {
        Execution::Completed
    }
}

impl Report<'_> {
    pub(in crate::clones) fn permits_success(&self) -> bool {
        self.scope.execution == Execution::NotRequested
            || (self.scope.execution == Execution::Completed
                && self.scope.coverage == Coverage::Complete)
    }

    pub(in crate::clones) fn render(&self) -> String {
        if self.scope.execution == Execution::NotRequested {
            return String::new();
        }
        let mut output = format!(
            "\nMutation analysis: {:?} coverage; {} findings; {} complete, {} unavailable functions.",
            self.scope.coverage,
            self.findings,
            self.scope.evaluated_functions,
            self.scope.unavailable_functions
        );
        for function in &self.functions {
            for rule in function
                .rules
                .iter()
                .filter(|r| r.outcome == Outcome::Finding)
            {
                output.push_str(&format!(
                    "\n{} {} at {} [bytes {}..{}]: at least {} sites/roots (limit {}).",
                    rule.rule_id,
                    function.name,
                    function.source_file.unwrap_or("<source unavailable>"),
                    function.fn_span.start,
                    function.fn_span.end,
                    rule.actual.unwrap_or(0),
                    rule.limit
                ));
            }
        }
        output
    }
}
