//! Finish fallible consumer work before explicit review publication.
use super::Context;
use crate::compiler::review;

pub(in crate::compiler) struct Consumers {
    pub(super) compiler_advisory: Option<crate::compiler::advisory::Report>,
    pub(super) review_prune: Option<review::prune::Report>,
    pub(super) review_status: Option<review::status::Report>,
}

impl Consumers {
    pub(in crate::compiler) fn new(ctx: &Context<'_>) -> Result<Self, String> {
        let store = review::workflow::load(ctx.args.review_file.as_deref())?;
        let compiler_advisory = crate::compiler::advisory::evaluate(ctx, store.as_ref())?;
        let reviews = review::workflow::Inputs {
            compiler_advisory: compiler_advisory.as_ref(),
        }
        .apply(ctx, store)?;
        Ok(Self {
            compiler_advisory: reviews.compiler.apply(compiler_advisory, ctx),
            review_status: reviews.status,
            review_prune: reviews.pruning,
        })
    }
}
