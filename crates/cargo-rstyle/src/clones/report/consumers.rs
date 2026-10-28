//! Finish fallible consumer work before explicit review publication.
use super::{Context, SourceReviews, locations};
use crate::clones::{analysis, planning, review};

pub(in crate::clones) struct Consumers<'a> {
    pub(super) compiler_advisory: Option<crate::clones::advisory::Report>,
    pub(super) pairs: analysis::pair_reports::Reports<'a>,
    pub(super) dispatch: analysis::dispatch_reports::Report<'a>,
    pub(super) capabilities: analysis::capability_reports::Report<'a>,
    pub(super) edit_planning: planning::Report,
    pub(super) guard_planning: Option<planning::guards::Report>,
    pub(super) edit_coordination: Option<planning::coordination::Report>,
    pub(super) parameter_planning: Option<planning::parameters::Report>,
    pub(super) analysis: analysis::Report<'a>,
    pub(super) mutation: analysis::mutation_reports::Report<'a>,
    pub(super) effects: analysis::effect_reports::Report<'a>,
    pub(super) accepted: Vec<locations::AcceptedPair<'a>>,
    pub(super) review: review::workflow::Report,
    pub(super) review_prune: Option<review::prune::Report>,
    pub(super) review_status: Option<review::status::Report>,
    pub(super) abstraction_review: review::local_workflow::Report,
    pub(super) numeric_reviews: NumericReviews,
    pub(super) source_reviews: SourceReviews,
}

impl<'a> Consumers<'a> {
    pub(in crate::clones) fn new(ctx: &Context<'a>) -> Result<Self, String> {
        let compiler_advisory = crate::clones::advisory::evaluate(ctx)?;
        let mut analysis = ctx.local_analysis();
        let guard_planning = planning::guards::evaluate(ctx, &analysis)?;
        let parameter_planning = planning::parameters::evaluate(ctx, &analysis)?;
        let pairs = ctx.pair_reports()?;
        let indices = ctx.declaration_indices()?;
        let dispatch = ctx.dispatch_report(&indices)?;
        let capabilities = ctx.capability_report(&indices)?;
        let edit_planning = ctx.edit_planning(&pairs.helper_proposals)?;
        let mut mutation = ctx.mutation_report()?;
        let mut effects = ctx.effect_report()?;
        let accepted = locations::accepted(ctx.handoff, ctx.matching)?;

        let reviews = review::workflow::Inputs {
            compiler_advisory: compiler_advisory.as_ref(),
            analysis: &analysis,
            mutation: &mutation,
            effects: &effects,
            helpers: &pairs.helper_proposals,
            dispatch: &dispatch,
            capabilities: &capabilities,
            indices: &indices,
        }
        .apply(ctx)?;
        let compiler_advisory = reviews.compiler.apply(compiler_advisory, ctx);
        mutation.hide_reviewed(&reviews.mutations);
        ctx.present_effects(&mut effects, &reviews.density);
        let source_reviews = SourceReviews::apply(reviews.sources, ctx.args, &mut analysis);
        Ok(Self {
            compiler_advisory,
            pairs,
            analysis,
            edit_coordination: coordinate(&edit_planning, &parameter_planning, &guard_planning),
            parameter_planning,
            guard_planning,
            dispatch,
            capabilities,
            edit_planning,
            mutation,
            numeric_reviews: NumericReviews::new(reviews.mutations, reviews.density),
            effects,
            accepted,
            review: reviews.helpers,
            abstraction_review: reviews.abstractions,
            source_reviews,
            review_status: reviews.status,
            review_prune: reviews.pruning,
        })
    }
}

/// Presentation decisions for completed numeric effect measurements.
#[derive(serde::Serialize)]
pub(in crate::clones) struct NumericReviews {
    pub density_review: review::density_workflow::Report,
    pub mutation_review: review::mutation_workflow::Report,
}

impl NumericReviews {
    fn new(
        mutation_review: review::mutation_workflow::Report,
        density_review: review::density_workflow::Report,
    ) -> Self {
        Self {
            mutation_review,
            density_review,
        }
    }
}

impl<'a> Context<'a> {
    fn present_effects(
        &self,
        effects: &mut analysis::effect_reports::Report<'a>,
        reviews: &review::density_workflow::Report,
    ) {
        effects.hide_reviewed(reviews);
        effects.select_changed(
            self.analysis_config
                .and_then(analysis::Config::changed_files),
            &self.capture.plan.root,
        );
    }
}

fn coordinate(
    edits: &planning::Report,
    parameters: &Option<planning::parameters::Report>,
    guards: &Option<planning::guards::Report>,
) -> Option<planning::coordination::Report> {
    planning::coordination::evaluate(edits, parameters.as_ref(), guards.as_ref())
}
