use super::*;

#[derive(Serialize)]
pub(super) struct Summary {
    pub execution: Execution,
    pub coverage: Coverage,
    evidence_execution: Execution,
    evidence_coverage: Coverage,
    population: &'static str,
    expected_functions: Option<usize>,
    assessed_functions: usize,
    pub evaluated_functions: usize,
    pub unavailable_functions: usize,
    pub policy_evaluated_functions: usize,
    pub excluded_functions: usize,
    pub policy_unavailable_functions: usize,
    reasons: Vec<String>,
}

pub(super) fn summary(
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
    let counts = Population::observe(functions);
    let evidence_execution = execution(
        functions.iter().map(|f| f.execution),
        requested,
        expected_functions.is_some(),
    );
    let evidence_coverage = coverage(
        requested,
        reasons.is_empty() && counts.unavailable == 0,
        evidence_execution,
    );
    let execution = execution(
        functions.iter().map(|f| f.policy.execution),
        requested,
        expected_functions.is_some(),
    );
    if requested && counts.policy_unavailable > 0 {
        reasons.push("Some owners have incomplete mutation policy or evidence; see original boundaries and policy decisions".into());
    }

    Summary {
        execution,
        coverage: coverage(requested, reasons.is_empty(), execution),
        evidence_execution,
        evidence_coverage,
        population: "captured function owners, independent of strict graph admission",
        expected_functions,
        assessed_functions: counts.assessed,
        evaluated_functions: counts.evaluated,
        unavailable_functions: counts.unavailable,
        policy_evaluated_functions: counts.policy_evaluated,
        excluded_functions: counts.excluded,
        policy_unavailable_functions: counts.policy_unavailable,
        reasons,
    }
}

fn coverage(requested: bool, complete: bool, execution: Execution) -> Coverage {
    if requested && complete && execution == Execution::Completed {
        Coverage::Complete
    } else {
        Coverage::Incomplete
    }
}

fn execution(
    mut outcomes: impl Iterator<Item = Execution>,
    requested: bool,
    captured: bool,
) -> Execution {
    if !requested {
        Execution::NotRequested
    } else if !captured {
        Execution::Failed
    } else if outcomes.any(|outcome| outcome == Execution::BudgetExhausted) {
        Execution::BudgetExhausted
    } else {
        Execution::Completed
    }
}

#[derive(Default)]
struct Population {
    assessed: usize,
    evaluated: usize,
    unavailable: usize,
    policy_evaluated: usize,
    excluded: usize,
    policy_unavailable: usize,
}

impl Population {
    fn observe(functions: &[FunctionReport<'_>]) -> Self {
        functions.iter().fold(Self::default(), |counts, function| {
            let evaluated = function.coverage == Coverage::Complete;
            let excluded = function.policy.excluded();
            let policy_evaluated = !excluded
                && function.policy.coverage == Coverage::Complete
                && function.policy.execution == Execution::Completed;

            Self {
                assessed: counts.assessed
                    + usize::from(function.execution != Execution::BudgetExhausted),
                evaluated: counts.evaluated + usize::from(evaluated),
                unavailable: counts.unavailable + usize::from(!evaluated),
                policy_evaluated: counts.policy_evaluated + usize::from(policy_evaluated),
                excluded: counts.excluded + usize::from(excluded),
                policy_unavailable: counts.policy_unavailable
                    + usize::from(!excluded && !policy_evaluated),
            }
        })
    }
}
