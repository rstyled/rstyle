//! Local dispatch advice uses source/signature evidence independently of strict pairs.
use super::declaration_indices::Indices;
use super::{Coverage, Execution, FactAvailability, Outcome};
use crate::clones::{Args, Function, FunctionKey, Handoff, capture};
use rstyle_pdg::consumer_facts::{Availability, dispatch};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct Report<'a> {
    schema: &'static str,
    detector: &'static str,
    #[serde(flatten)]
    scope: Summary,
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
    proposal: Option<dispatch::Candidate>,
    reason: Option<String>,
}

pub(in crate::clones) fn evaluate<'a>(
    handoff: &'a Handoff,
    capture: &capture::Capture,
    args: &Args,
    indices: &Indices,
) -> Result<Report<'a>, String> {
    let functions = if args.enum_dispatch {
        evaluate_functions(
            handoff,
            args.max_analysis_functions.unwrap_or(usize::MAX),
            indices,
        )?
    } else {
        Vec::new()
    };
    Ok(summarize(functions, handoff, capture, args.enum_dispatch))
}

fn evaluate_functions<'a>(
    handoff: &'a Handoff,
    limit: usize,
    indices: &Indices,
) -> Result<Vec<FunctionReport<'a>>, String> {
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
                ))
            } else {
                let assessment = indices
                    .get(function.key.unit())?
                    .evaluate(&function.facts)?;
                Ok(assess(function, assessment))
            }
        })
        .collect()
}

fn assess(function: &Function, assessment: dispatch::Assessment) -> FunctionReport<'_> {
    if assessment.budget_exhausted {
        let reason = match &assessment.availability {
            Availability::Unknown { reason }
            | Availability::Unsupported { reason }
            | Availability::NotApplicable { reason } => reason.as_str(),
            Availability::Known(_) => "Dispatch evidence budget exhausted",
        };
        return unavailable(function, Execution::BudgetExhausted, reason);
    }
    let (availability, proposal, reason) = match assessment.availability {
        Availability::Known(proposal) => (FactAvailability::Known, proposal, assessment.reason),
        Availability::Unknown { reason } => (FactAvailability::Unknown, None, Some(reason)),
        Availability::Unsupported { reason } => (FactAvailability::Unsupported, None, Some(reason)),
        Availability::NotApplicable { reason } => {
            (FactAvailability::NotApplicable, None, Some(reason))
        }
    };
    let outcome = if proposal.is_some() {
        Outcome::Finding
    } else if availability == FactAvailability::Known {
        Outcome::EvaluatedNoFinding
    } else {
        Outcome::NotEvaluated
    };
    FunctionReport {
        key: function.key,
        name: &function.facts.name,
        source_file: source_file(function),
        fn_span: function.facts.fn_span,
        execution: Execution::Completed,
        outcome,
        coverage: if outcome == Outcome::NotEvaluated {
            Coverage::Incomplete
        } else {
            Coverage::Complete
        },
        availability,
        proposal,
        reason,
    }
}

fn source_file(function: &Function) -> Option<&str> {
    function
        .facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
}

fn unavailable<'a>(
    function: &'a Function,
    execution: Execution,
    reason: &str,
) -> FunctionReport<'a> {
    FunctionReport {
        key: function.key,
        name: &function.facts.name,
        source_file: source_file(function),
        fn_span: function.facts.fn_span,
        execution,
        outcome: Outcome::NotEvaluated,
        coverage: Coverage::Incomplete,
        availability: FactAvailability::Unknown,
        proposal: None,
        reason: Some(reason.into()),
    }
}

fn summarize<'a>(
    functions: Vec<FunctionReport<'a>>,
    handoff: &Handoff,
    capture: &capture::Capture,
    requested: bool,
) -> Report<'a> {
    Report {
        schema: "rstyle.enum-dispatch/1",
        detector: "local-enum-dispatch/1",
        scope: summary(&functions, handoff, capture, requested),
        findings: functions
            .iter()
            .filter(|f| f.outcome == Outcome::Finding)
            .count(),
        functions,
        limitations: &[
            "Advisory local two-variant dispatch with shared-self u32 method signatures; not a safe rewrite or equivalence proof.",
            "Method body effects, purity, lifetimes and drop behavior require manual review.",
            "Legacy suppressions and review entries are not applied; no legacy capability is retired.",
        ],
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
        .filter(|f| f.outcome != Outcome::NotEvaluated)
        .count();
    let unavailable_functions = functions.len() - evaluated_functions;
    if requested && unavailable_functions > 0 {
        reasons.push(
            "Some captured owners lack complete dispatch prerequisites; see function reasons"
                .into(),
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
            "\nEnum dispatch: {:?} coverage; {} findings; {} evaluated, {} unavailable functions.",
            self.scope.coverage,
            self.findings,
            self.scope.evaluated_functions,
            self.scope.unavailable_functions
        );
        for function in &self.functions {
            if function.proposal.is_some() {
                output.push_str(&format!(
                    "\nAdvisory enum_dispatch {} {} at {} [bytes {}..{}].",
                    function.key,
                    function.name,
                    function.source_file.unwrap_or("<source unavailable>"),
                    function.fn_span.start,
                    function.fn_span.end
                ));
            }
        }
        output
    }
}
