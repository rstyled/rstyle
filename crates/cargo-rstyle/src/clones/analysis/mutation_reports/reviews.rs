//! Borrow completed mutation counts and policy; review never reruns inspection.
use super::*;
use crate::clones::review::{mutation_rules, mutation_workflow};

pub(in crate::clones) struct Review<'a> {
    pub key: FunctionKey,
    pub rules: Vec<RuleReview>,
    pub sites_per_root: &'a [usize],
}

pub(in crate::clones) struct RuleReview {
    pub policy: mutation_rules::Policy,
    pub visible: bool,
    pub complete: bool,
}

impl Report<'_> {
    pub(in crate::clones) fn review_scope_complete(&self) -> bool {
        self.scope.coverage == Coverage::Complete && self.scope.execution == Execution::Completed
    }

    pub(in crate::clones) fn reviews(&self) -> impl Iterator<Item = Review<'_>> {
        self.functions.iter().map(|function| Review {
            key: function.location.key,
            sites_per_root: function
                .evidence
                .as_ref()
                .map_or(&[], |e| &e.sites_per_root),
            rules: function
                .policy
                .rule_decisions
                .iter()
                .map(|rule| RuleReview {
                    policy: match rule.rule_id {
                        "mutation_per_target" => mutation_rules::Policy::MutationPerTarget {
                            max_mutations_per_target: rule.limit,
                        },
                        _ => mutation_rules::Policy::MutatedTargets {
                            max_mutated_targets: rule.limit,
                        },
                    },
                    visible: rule.outcome == Outcome::Finding,
                    complete: rule.disposition == decisions::Disposition::Evaluated
                        && function.coverage == Coverage::Complete
                        && function.execution == Execution::Completed,
                })
                .collect(),
        })
    }

    pub(in crate::clones) fn hide_reviewed(&mut self, review: &mutation_workflow::Report) {
        self.review_hidden_findings = self
            .functions
            .iter_mut()
            .map(|function| function.hide_reviewed(review))
            .sum();
        self.visible_findings = self
            .visible_findings
            .saturating_sub(self.review_hidden_findings);
    }
}

impl FunctionReport<'_> {
    fn hide_reviewed(&mut self, review: &mutation_workflow::Report) -> usize {
        self.review_hidden = ["mutation_per_target", "mutated_targets"]
            .map(|rule| review.hidden_for(self.location.key, rule));
        self.policy
            .rule_decisions
            .iter()
            .filter(|rule| {
                rule.outcome == Outcome::Finding
                    && review.hidden_for(self.location.key, rule.rule_id)
            })
            .count()
    }
}
