//! Exact source-local dismissal; loading a snapshot never grants current evidence.
mod model;
use super::Locator;
pub(crate) use model::*;

type Error = super::ReviewError;

pub(super) fn validate_locator(locator: &Locator) -> Result<(), Error> {
    if !super::relative_source(&locator.package.identity)
        || locator.package.identity.rsplit('/').next() != Some("Cargo.toml")
        || locator.package.name.is_empty()
        || locator.package.version.is_empty()
        || !super::relative_source(&locator.source)
        || locator.qualified_name.is_empty()
        || locator.declaration_kind.is_empty()
        || locator.discriminator.is_empty()
    {
        return Err(Error::InvalidLocator);
    }
    Ok(())
}

pub(super) fn validate_context(context: &BuildContext) -> Result<(), Error> {
    let target = &context.compilation_target;
    if [
        &context.compiler_commit,
        &context.edition,
        &context.data_layout,
        &context.target_triple,
        &context.opt_level,
        &context.panic_strategy,
        &context.extraction_policy,
        &target.name,
    ]
    .iter()
    .any(|value| value.is_empty())
        || context.pointer_width == 0
        || target.kind.is_empty()
        || target.kind.iter().any(String::is_empty)
        || !canonical(&target.kind)
        || !canonical(&context.features)
        || !canonical(&context.cfgs)
    {
        return Err(Error::InvalidContext);
    }
    Ok(())
}

fn canonical(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}
