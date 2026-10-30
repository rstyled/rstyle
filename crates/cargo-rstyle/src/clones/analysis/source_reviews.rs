//! Review adapters borrow completed source assessments and retained signature facts.
use super::{Coverage, FunctionReport, Report, RuleReview, Settings, SourceReview};
use crate::clones::review;

#[derive(Default)]
pub(super) struct Hidden {
    parameters: bool,
    lengths: bool,
    spacings: bool,
}

impl Hidden {
    pub(super) fn contains(&self, rule_id: &str) -> bool {
        match rule_id {
            "fn_params" => self.parameters,
            "fn_length" => self.lengths,
            "fn_spacing" => self.spacings,
            _ => false,
        }
    }
}

impl Report<'_> {
    pub(in crate::clones) fn source_baseline_valid(&self) -> bool {
        self.functions.iter().all(|function| {
            function
                .assessments
                .iter()
                .all(|assessment| assessment.rule_id != "invalid_suppression")
        })
    }

    pub(in crate::clones) fn spacing_policy(&self) -> Option<usize> {
        self.effective_config
            .as_ref()
            .and_then(|settings| settings.min_statements)
    }

    pub(in crate::clones) fn source_reviews(&self) -> impl Iterator<Item = SourceReview<'_>> {
        self.effective_config.iter().flat_map(move |settings| {
            self.functions
                .iter()
                .filter(|function| !function.outside_source_rules)
                .map(move |function| function.source_review(settings, self.changed_scope.is_none()))
        })
    }

    pub(in crate::clones) fn hide_reviewed_source_rules(
        &mut self,
        reviews: &review::source_workflow::Reports,
    ) {
        for function in &mut self.functions {
            function.source_review_hidden = Hidden {
                parameters: reviews.parameters.hidden_for(function.key),
                lengths: reviews.lengths.hidden_for(function.key),
                spacings: reviews.spacings.hidden_for(function.key),
            };
        }
    }
}

impl FunctionReport<'_> {
    pub(super) fn parameter_source_complete(&self) -> bool {
        !self.outside_source_rules && self.source_rule_review("fn_params").complete
    }

    fn source_review(&self, settings: &Settings<'_>, unfiltered: bool) -> SourceReview<'_> {
        SourceReview {
            key: self.key,
            selected: unfiltered || self.changed_file == Some(true),
            signature: self.signature,
            count_self: settings.count_self,
            parameter: self.source_rule_review("fn_params"),
            length: self.source_rule_review("fn_length"),
            spacing: self.source_rule_review("fn_spacing"),
            min_statements: settings.min_statements,
        }
    }

    fn source_rule_review(&self, rule_id: &str) -> RuleReview<'_> {
        let assessment = self.assessments.iter().find(|rule| rule.rule_id == rule_id);
        RuleReview {
            suppressed: self
                .source_suppression
                .as_ref()
                .is_some_and(|suppression| suppression.allows(rule_id)),
            assessment,
            complete: assessment.is_some_and(|rule| rule.coverage == Coverage::Complete)
                && self.source_policy_complete(),
            visible: assessment.is_some_and(|rule| self.visible(rule)),
        }
    }
}

impl super::Config {
    pub(in crate::clones) fn source_baseline_header(
        &self,
    ) -> Option<(review::source_baseline::Policy, Vec<String>)> {
        let source = self.source.as_ref()?;
        let settings = Settings::new(source);
        Some((
            review::source_baseline::Policy {
                parameters: review::source_rules::Policy {
                    max_params: settings.max_params,
                    count_self: settings.count_self,
                },
                lengths: review::length_rules::Policy {
                    max_lines: settings.max_lines,
                },
                spacings: settings
                    .min_statements
                    .filter(|minimum| *minimum > 0)
                    .map(|min_statements| review::spacing_rules::Policy { min_statements }),
            },
            settings.exclude.to_vec(),
        ))
    }
}
