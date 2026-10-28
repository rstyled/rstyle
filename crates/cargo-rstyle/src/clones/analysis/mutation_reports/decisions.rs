//! Visibility policy never rewrites the original mutation assessment.
use super::{
    super::{Assessment, Coverage, Execution, source_exclusions, source_suppressions},
    Outcome,
    policy::Rule,
};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Policy {
    pub coverage: Coverage,
    pub execution: Execution,
    pub source_exclusion: source_exclusions::Report,
    pub source_suppression: source_suppressions::Report,
    pub rule_decisions: Vec<RuleDecision>,
    pub invalid_suppression: Option<Assessment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Disposition {
    Evaluated,
    Suppressed,
    Excluded,
    Unavailable,
}

#[derive(Serialize)]
pub(super) struct RuleDecision {
    pub rule_id: &'static str,
    pub outcome: Outcome,
    pub actual: Option<usize>,
    pub limit: usize,
    pub disposition: Disposition,
    pub reason: Option<&'static str>,
    #[serde(skip)]
    witnessed_finding: bool,
}

pub(super) struct RawStatus {
    pub complete: bool,
    pub execution: Execution,
}

pub(super) fn evaluate(
    exclusion: source_exclusions::Report,
    suppression: source_suppressions::Report,
    rules: &[Rule],
    raw: RawStatus,
) -> Policy {
    let excluded = exclusion.excluded();
    let complete = excluded || (raw.complete && exclusion.complete() && suppression.complete());
    let rule_decisions = rules
        .iter()
        .map(|rule| decision(rule, &exclusion, &suppression, raw.complete))
        .collect();
    let invalid_suppression = suppression.invalid_assessment();

    Policy {
        coverage: if complete {
            Coverage::Complete
        } else {
            Coverage::Incomplete
        },
        execution: if excluded {
            Execution::Completed
        } else {
            raw.execution
        },
        source_exclusion: exclusion,
        source_suppression: suppression,
        rule_decisions,
        invalid_suppression,
    }
}

fn decision(
    rule: &Rule,
    exclusion: &source_exclusions::Report,
    suppression: &source_suppressions::Report,
    raw_complete: bool,
) -> RuleDecision {
    let (disposition, reason) = disposition(rule.rule_id, exclusion, suppression, raw_complete);
    let skipped = matches!(disposition, Disposition::Suppressed | Disposition::Excluded);
    RuleDecision {
        rule_id: rule.rule_id,
        outcome: if skipped {
            Outcome::NotEvaluated
        } else {
            rule.outcome
        },
        actual: if disposition == Disposition::Excluded {
            None
        } else {
            rule.actual
        },
        limit: rule.limit,
        disposition,
        reason,
        witnessed_finding: rule.outcome == Outcome::Finding,
    }
}

fn disposition(
    rule: &str,
    exclusion: &source_exclusions::Report,
    suppression: &source_suppressions::Report,
    raw_complete: bool,
) -> (Disposition, Option<&'static str>) {
    if exclusion.excluded() {
        return (
            Disposition::Excluded,
            Some("Original source path is excluded by workspace policy"),
        );
    }
    if suppression.allows_mutation(rule) {
        return (
            Disposition::Suppressed,
            Some("Valid captured suppression names this mutation rule"),
        );
    }
    if !exclusion.complete() || !suppression.complete() {
        return (
            Disposition::Unavailable,
            Some("Captured mutation visibility policy is unavailable"),
        );
    }
    if !raw_complete {
        return (
            Disposition::Unavailable,
            Some("Original mutation coverage is incomplete"),
        );
    }

    (Disposition::Evaluated, None)
}

impl Policy {
    pub(super) fn excluded(&self) -> bool {
        self.source_exclusion.excluded()
    }

    fn counts(&self) -> FindingCounts {
        let initial = FindingCounts {
            visible: usize::from(self.invalid_suppression.is_some()),
            ..FindingCounts::default()
        };

        self.rule_decisions
            .iter()
            .fold(initial, |counts, rule| FindingCounts {
                visible: counts.visible + usize::from(rule.outcome == Outcome::Finding),
                suppressed: counts.suppressed
                    + usize::from(
                        rule.witnessed_finding && rule.disposition == Disposition::Suppressed,
                    ),
                excluded: counts.excluded
                    + usize::from(
                        rule.witnessed_finding && rule.disposition == Disposition::Excluded,
                    ),
            })
    }
}

#[derive(Default)]
pub(super) struct FindingCounts {
    pub visible: usize,
    pub suppressed: usize,
    pub excluded: usize,
}

impl FindingCounts {
    pub(super) fn observe<'a>(policies: impl Iterator<Item = &'a Policy>) -> Self {
        policies
            .map(Policy::counts)
            .fold(Self::default(), |sum, counts| Self {
                visible: sum.visible + counts.visible,
                suppressed: sum.suppressed + counts.suppressed,
                excluded: sum.excluded + counts.excluded,
            })
    }
}

#[cfg(test)]
mod tests;
