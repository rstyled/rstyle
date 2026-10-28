//! Finish fallible consumer work before explicit review publication.
use super::{Context, locations};
use crate::clones::{analysis, planning, review};

pub(in crate::clones) struct Consumers<'a> {
    pub(super) pairs: analysis::pair_reports::Reports<'a>,
    pub(super) dispatch: analysis::dispatch_reports::Report<'a>,
    pub(super) capabilities: analysis::capability_reports::Report<'a>,
    pub(super) edit_planning: planning::Report,
    pub(super) mutation: analysis::mutation_reports::Report<'a>,
    pub(super) effects: analysis::effect_reports::Report<'a>,
    pub(super) accepted: Vec<locations::AcceptedPair<'a>>,
    pub(super) review: review::workflow::Report,
    pub(super) abstraction_review: review::local_workflow::Report,
}

impl<'a> Consumers<'a> {
    pub(in crate::clones) fn new(ctx: &Context<'a>) -> Result<Self, String> {
        let pairs = ctx.pair_reports()?;
        let indices = ctx.declaration_indices()?;
        let dispatch = ctx.dispatch_report(&indices)?;
        let capabilities = ctx.capability_report(&indices)?;
        let edit_planning = ctx.edit_planning(&pairs.helper_proposals)?;
        let mutation = ctx.mutation_report()?;
        let effects = ctx.effect_report()?;
        let accepted = locations::accepted(ctx.handoff, ctx.matching)?;

        let (review, abstraction_review) = review::workflow::apply(
            review::workflow::Inputs {
                helpers: &pairs.helper_proposals,
                dispatch: &dispatch,
                capabilities: &capabilities,
                indices: &indices,
            },
            ctx,
        )?;
        Ok(Self {
            pairs,
            dispatch,
            capabilities,
            edit_planning,
            mutation,
            effects,
            accepted,
            review,
            abstraction_review,
        })
    }
}
