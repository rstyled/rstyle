//! Review presentation borrows completed guard assessments; raw advice stays intact.
use super::{FunctionReport, Report, guards};
use crate::clones::{FunctionKey, review};

pub(in crate::clones) struct GuardReview<'a> {
    pub key: FunctionKey,
    pub source: Option<&'a str>,
    pub assessment: &'a guards::GuardAssessment,
}

impl Report<'_> {
    pub(in crate::clones) fn guard_reviews(&self) -> impl Iterator<Item = GuardReview<'_>> {
        self.functions.iter().filter_map(|function| {
            Some(GuardReview {
                key: function.key,
                source: function.guard_review_source,
                assessment: function.guard_clause.as_ref()?,
            })
        })
    }

    pub(in crate::clones) fn hide_reviewed_guards(
        &mut self,
        reviews: &review::guard_workflow::Report,
    ) {
        for function in &mut self.functions {
            function.guard_review_hidden = function
                .guard_clause
                .iter()
                .flat_map(|guard| guard.opportunity.iter().chain(&guard.closure_opportunities))
                .enumerate()
                .filter_map(|(index, _)| reviews.hidden_for(function.key, index).then_some(index))
                .collect();
        }
    }
}

impl FunctionReport<'_> {
    pub(super) fn visible_guard_finding(&self) -> bool {
        self.guard_clause
            .iter()
            .flat_map(|guard| guard.opportunity.iter().chain(&guard.closure_opportunities))
            .enumerate()
            .any(|(index, _)| !self.guard_review_hidden.contains(&index))
    }
}
