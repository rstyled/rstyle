//! Review adapters borrow completed source assessments and retained signature facts.
use super::{Coverage, FunctionReport, ParameterReview, Report, Settings};
use crate::clones::review;

impl Report<'_> {
    pub(in crate::clones) fn parameter_reviews(&self) -> impl Iterator<Item = ParameterReview<'_>> {
        self.effective_config.iter().flat_map(move |settings| {
            self.functions.iter().map(move |function| {
                function.parameter_review(settings, self.changed_scope.is_none())
            })
        })
    }

    pub(in crate::clones) fn hide_reviewed_parameters(
        &mut self,
        reviews: &review::source_workflow::Report,
    ) {
        for function in &mut self.functions {
            function.parameter_review_hidden = reviews.hidden_for(function.key);
        }
    }
}

impl FunctionReport<'_> {
    pub(super) fn parameter_source_complete(&self) -> bool {
        self.assessments.iter().any(|assessment| {
            assessment.rule_id == "fn_params" && assessment.coverage == Coverage::Complete
        }) && self.source_policy_complete()
    }

    fn parameter_review(&self, settings: &Settings<'_>, unfiltered: bool) -> ParameterReview<'_> {
        let assessment = self
            .assessments
            .iter()
            .find(|rule| rule.rule_id == "fn_params");
        ParameterReview {
            key: self.key,
            assessment,
            signature: self.signature,
            max_params: settings.max_params,
            count_self: settings.count_self,
            complete: self.parameter_source_complete(),
            visible: assessment.is_some_and(|assessment| self.visible(assessment)),
            selected: unfiltered || self.changed_file == Some(true),
        }
    }
}
