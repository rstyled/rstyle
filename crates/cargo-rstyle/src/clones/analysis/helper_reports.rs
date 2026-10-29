//! Helper scope includes explicit missing candidate-input coverage.
use super::{Coverage, Execution, Outcome, helpers};
use crate::clones::{Function, FunctionKey, Handoff};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct Report<'a> {
    schema: &'static str,
    detector: Detector,
    policy: &'static helpers::Policy,
    execution: Execution,
    coverage: Coverage,
    population: &'static str,
    expected_pairs: Option<usize>,
    assessed_pairs: usize,
    evaluated_pairs: usize,
    unavailable_pairs: usize,
    pub findings: usize,
    reasons: Vec<&'static str>,
    unavailable_functions: Vec<UnavailableFunction<'a>>,
    pairs: Vec<Pair<'a>>,
    limitations: &'static [&'static str],
}

#[derive(Serialize)]
struct Detector {
    id: &'static str,
    version: u32,
}

#[derive(Serialize)]
pub(super) struct Pair<'a> {
    pub first: &'a str,
    pub second: &'a str,
    #[serde(flatten)]
    pub assessment: helpers::Assessment<'a>,
}

#[derive(Serialize)]
struct UnavailableFunction<'a> {
    key: &'a FunctionKey,
    name: &'a str,
    outcome: &'a str,
    reasons: &'a [String],
}

impl<'a> From<&'a Function> for UnavailableFunction<'a> {
    fn from(function: &'a Function) -> Self {
        Self {
            key: &function.key,
            name: function.facts.view().name,
            outcome: &function.outcome,
            reasons: &function.reasons,
        }
    }
}

pub(super) fn summarize<'a>(
    pairs: Vec<Pair<'a>>,
    handoff: &'a Handoff,
    requested: bool,
    matching_complete: bool,
) -> Report<'a> {
    let unavailable_functions = unavailable_functions(handoff, requested);
    let unavailable_pairs = count(&pairs, Outcome::NotEvaluated);
    let complete = requested
        && matching_complete
        && unavailable_pairs == 0
        && unavailable_functions.is_empty();
    let execution = execution(&pairs, requested);
    let reasons = scope_reasons(
        matching_complete,
        requested,
        unavailable_pairs,
        unavailable_functions.len(),
    );

    Report {
        schema: "rstyle.helper-proposals/1",
        detector: Detector {
            id: "scalar-expression-helper",
            version: 1,
        },
        policy: &helpers::POLICY,
        execution,
        coverage: if complete {
            Coverage::Complete
        } else {
            Coverage::Incomplete
        },
        population: "accepted strict pairs; nonadmitted captured owners are unavailable potential members",
        expected_pairs: (requested && matching_complete).then_some(pairs.len()),
        assessed_pairs: pairs.len(),
        evaluated_pairs: pairs.len() - unavailable_pairs,
        unavailable_pairs,
        findings: count(&pairs, Outcome::Finding),
        reasons,
        unavailable_functions,
        pairs,
        limitations: &[
            "Advisory whole-function u32 expression helpers, not automatic safe rewrites.",
            "Integer overflow and call-site evaluation behavior require manual review.",
            "Each proposal covers exactly two members; connected pairs do not establish an all-member group.",
            "Ordered correspondence is retained in pair_explanations; endpoint keys join accepted_pairs source locations.",
            "Legacy suppressions and review entries are not applied; no legacy capability is retired.",
        ],
    }
}

fn unavailable_functions(handoff: &Handoff, requested: bool) -> Vec<UnavailableFunction<'_>> {
    if !requested {
        return Vec::new();
    }
    handoff
        .assessed
        .iter()
        .filter(|function| function.outcome != "admitted")
        .map(UnavailableFunction::from)
        .collect()
}

fn count(pairs: &[Pair<'_>], outcome: Outcome) -> usize {
    pairs
        .iter()
        .filter(|pair| pair.assessment.outcome == outcome)
        .count()
}

fn execution(pairs: &[Pair<'_>], requested: bool) -> Execution {
    if !requested {
        Execution::NotRequested
    } else if pairs
        .iter()
        .any(|pair| pair.assessment.execution == Execution::BudgetExhausted)
    {
        Execution::BudgetExhausted
    } else {
        Execution::Completed
    }
}

fn scope_reasons(
    matching_complete: bool,
    requested: bool,
    unavailable_pairs: usize,
    unavailable_functions: usize,
) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if requested && !matching_complete {
        reasons.push("Matching is incomplete; the full accepted-pair denominator is unknown");
    }
    if unavailable_pairs > 0 {
        reasons.push("Some accepted pairs lack complete helper prerequisites; see pair reasons");
    }
    if unavailable_functions > 0 {
        reasons.push("Some captured owners could not enter strict matching; potential helper members remain unavailable");
    }

    reasons
}

impl Report<'_> {
    pub(in crate::clones) fn finding_pairs(&self) -> impl Iterator<Item = (&str, &str)> {
        self.pairs.iter().filter_map(|pair| {
            pair.assessment
                .proposal
                .as_ref()
                .map(|_| (pair.first, pair.second))
        })
    }

    pub(in crate::clones) fn review_pair_without_finding(&self, first: &str, second: &str) -> bool {
        self.pairs.iter().any(|pair| {
            ((pair.first == first && pair.second == second)
                || (pair.first == second && pair.second == first))
                && pair.assessment.execution == Execution::Completed
                && pair.assessment.outcome == Outcome::EvaluatedNoFinding
        })
    }

    pub(in crate::clones) fn scope_complete(&self) -> bool {
        self.execution == Execution::Completed && self.coverage == Coverage::Complete
    }

    pub(super) fn permits_success(&self) -> bool {
        self.execution == Execution::NotRequested
            || (self.execution == Execution::Completed && self.coverage == Coverage::Complete)
    }

    pub(super) fn render(&self) -> String {
        if self.execution == Execution::NotRequested {
            return String::new();
        }
        let mut output = format!(
            "\nHelper proposals: {:?} coverage; {} findings; {} evaluated, {} unavailable pairs; {} unavailable functions.",
            self.coverage,
            self.findings,
            self.evaluated_pairs,
            self.unavailable_pairs,
            self.unavailable_functions.len()
        );
        for pair in &self.pairs {
            if let Some(proposal) = &pair.assessment.proposal {
                output.push_str(&format!(
                    "\nAdvisory helper {} <> {}: {}",
                    pair.first,
                    pair.second,
                    proposal.advisory()
                ));
            }
        }
        output
    }
}
