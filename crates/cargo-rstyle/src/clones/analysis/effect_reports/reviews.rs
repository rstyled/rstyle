//! Reviews borrow completed measurements and never recapture or certify effects.
use super::*;
use crate::clones::review::{density_rules, density_workflow};

pub(in crate::clones) struct Review {
    pub key: FunctionKey,
    pub complete: bool,
    pub visible: bool,
    pub policy: density_rules::Policy,
    pub measurement: Option<density_rules::Measurement>,
}

impl Report<'_> {
    pub(in crate::clones) fn review_scope_complete(&self) -> bool {
        self.scope.coverage == Coverage::Complete && self.scope.execution == Execution::Completed
    }

    pub(in crate::clones) fn reviews(&self) -> impl Iterator<Item = Review> {
        self.functions.iter().filter_map(|function| {
            Some(
                function
                    .density
                    .as_ref()?
                    .review(function.key, self.density_config?),
            )
        })
    }

    pub(in crate::clones) fn hide_reviewed(&mut self, review: &density_workflow::Report) {
        let hidden = hide_functions(&mut self.functions, review);
        self.publish_hidden(hidden);
    }

    fn publish_hidden(&mut self, hidden: usize) {
        self.review_hidden_findings = hidden;
        self.visible_findings = self
            .visible_findings
            .saturating_sub(self.review_hidden_findings);
    }
}

fn hide_functions(
    functions: &mut [FunctionReport<'_>],
    review: &density_workflow::Report,
) -> usize {
    functions
        .iter_mut()
        .filter_map(|function| {
            Some(
                function
                    .density
                    .as_mut()?
                    .hide_reviewed(review.hidden_for(function.key)),
            )
        })
        .sum()
}
