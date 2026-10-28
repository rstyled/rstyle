//! Finish fallible consumer work before explicit review publication.
use super::{Context, SourceReviews, locations};
use crate::clones::{analysis, planning, review};

pub(in crate::clones) struct Consumers<'a> {
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
    pub(super) source_reviews: SourceReviews,
}

impl<'a> Consumers<'a> {
    pub(in crate::clones) fn new(ctx: &Context<'a>) -> Result<Self, String> {
        let mut analysis = ctx.local_analysis();
        let guard_planning = planning::guards::evaluate(ctx, &analysis)?;
        let parameter_planning = planning::parameters::evaluate(ctx, &analysis)?;
        let pairs = ctx.pair_reports()?;
        let indices = ctx.declaration_indices()?;
        let dispatch = ctx.dispatch_report(&indices)?;
        let capabilities = ctx.capability_report(&indices)?;
        let edit_planning = ctx.edit_planning(&pairs.helper_proposals)?;
        let edit_coordination = planning::coordination::evaluate(
            &edit_planning,
            parameter_planning.as_ref(),
            guard_planning.as_ref(),
        );
        let mutation = ctx.mutation_report()?;
        let effects = ctx.effect_report()?;
        let accepted = locations::accepted(ctx.handoff, ctx.matching)?;

        let reviews = review::workflow::Inputs {
            analysis: &analysis,
            helpers: &pairs.helper_proposals,
            dispatch: &dispatch,
            capabilities: &capabilities,
            indices: &indices,
        }
        .apply(ctx)?;
        analysis.hide_reviewed_source_rules(&reviews.sources);
        let source_reviews = SourceReviews::new(
            reviews.sources,
            ctx.args,
            analysis.spacing_policy().is_some(),
        );
        Ok(Self {
            pairs,
            analysis,
            parameter_planning,
            guard_planning,
            edit_coordination,
            dispatch,
            capabilities,
            edit_planning,
            mutation,
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
