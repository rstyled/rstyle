//! Finish fallible consumer work before explicit review publication.
use super::Context;
use crate::clones::review;

pub(in crate::clones) struct Consumers {
    pub(super) compiler_advisory: Option<crate::clones::advisory::Report>,
    pub(super) review_prune: Option<review::prune::Report>,
    pub(super) review_status: Option<review::status::Report>,
}

impl Consumers {
    pub(in crate::clones) fn new(ctx: &Context<'_>) -> Result<Self, String> {
        let store = review::workflow::load(ctx.args.review_file.as_deref())?;
        let compiler_advisory = crate::clones::advisory::evaluate(ctx, store.as_ref())?;
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
