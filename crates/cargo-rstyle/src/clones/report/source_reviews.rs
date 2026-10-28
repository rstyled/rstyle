//! Present independent source review families without filtering raw analysis.
use crate::clones::{
    Args,
    analysis::Completion,
    review::{source_baseline, source_workflow},
};
use serde::Serialize;

#[derive(Serialize)]
pub(in crate::clones) struct SourceReviews {
    #[serde(skip_serializing_if = "Option::is_none")]
    parameter_review: Option<source_workflow::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::clones) source_baseline: Option<source_baseline::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    length_review: Option<source_workflow::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spacing_review: Option<source_workflow::Report>,
}

impl SourceReviews {
    pub(super) fn apply(
        reports: source_workflow::Reports,
        args: &Args,
        analysis: &mut crate::clones::analysis::Report<'_>,
    ) -> Self {
        analysis.hide_reviewed_source_rules(&reports);
        let spacing = analysis.spacing_policy().is_some();
        let requested = args.source_rules && args.review_file.is_some();
        Self {
            source_baseline: reports.baseline,
            parameter_review: requested.then_some(reports.parameters),
            length_review: requested.then_some(reports.lengths),
            spacing_review: (requested && spacing).then_some(reports.spacings),
        }
    }

    fn iter(&self) -> impl Iterator<Item = &source_workflow::Report> {
        [
            self.parameter_review.as_ref(),
            self.length_review.as_ref(),
            self.spacing_review.as_ref(),
        ]
        .into_iter()
        .flatten()
    }

    pub(in crate::clones) fn hidden_selected_findings(&self) -> usize {
        self.iter()
            .map(|review| review.hidden_selected_findings())
            .sum()
    }

    pub(super) fn render(&self) -> String {
        let mut output = String::new();
        for review in self.iter() {
            output.push_str(&review.render());
        }
        if let Some(baseline) = &self.source_baseline {
            output.push_str(&baseline.render());
        }
        output
    }
}

impl Completion for SourceReviews {
    fn permits_success(&self) -> bool {
        self.iter().all(Completion::permits_success)
            && self
                .source_baseline
                .as_ref()
                .is_none_or(source_baseline::Report::permits_success)
    }
}

impl SourceReviews {
    pub(in crate::clones) fn baseline_gate_fails(&self) -> bool {
        self.source_baseline
            .iter()
            .any(source_baseline::Report::gate_fails)
    }
}
