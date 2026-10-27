//! Complete bounded From forwarder proof, separate from concrete-owner admission.
mod association;
pub(super) mod materialize;
mod source;
#[cfg(test)]
mod tests;

use super::body::{model::Observation, preserved};
use crate::{HirFnFacts, nominal_boundary::census::Census};
use serde::Deserialize;
use std::sync::Arc;

type Result<T> = std::result::Result<T, String>;
const NAME: &str = "<util::search::Input<'h> as core::convert::From<&'h H>>::from";

pub(super) fn verify(
    forwarder: super::ReferenceObservation<'_>,
    constructor: super::ReferenceObservation<'_>,
    haystack: &str,
) -> Result<()> {
    let super::ReferenceObservation {
        canonical,
        observation,
    } = forwarder;
    let super::ReferenceObservation {
        canonical: constructor,
        observation: constructor_observation,
    } = constructor;
    super::verify_source(canonical)?;
    super::verify_source(constructor)?;
    super::body::verify(constructor, constructor_observation, haystack)?;
    same_context(canonical, constructor)?;

    let record = Observation::deserialize(observation).map_err(|error| error.to_string())?;
    let target =
        Observation::deserialize(constructor_observation).map_err(|error| error.to_string())?;
    if canonical.name != NAME
        || record.status != "observed"
        || record.request.function != NAME
        || record.request.haystack != haystack
        || !matches!(haystack, "str" | "[u8]")
        || !record.conversions.is_empty()
        || record.calls.len() != 1
    {
        return Err("Forwarder proof requires its complete exact From instance".into());
    }
    preserved::snapshot(canonical, &record, observation, haystack)?;
    let call = source::verify(canonical)?;
    source::projection(&record, &target, call, haystack)?;
    association::verify(canonical, &record, &target, call)?;
    Ok(())
}

fn same_context(canonical: &HirFnFacts, constructor: &HirFnFacts) -> Result<()> {
    let from = &canonical.build;
    let new = &constructor.build;
    if from.producer_revision != new.producer_revision
        || from.rustc_commit != new.rustc_commit
        || from.rustc_version != new.rustc_version
        || from.target_triple != new.target_triple
        || from.pointer_width != new.pointer_width
        || from.data_layout != new.data_layout
        || from.cfgs != new.cfgs
        || from.features != new.features
        || from.edition != new.edition
        || from.overflow_checks != new.overflow_checks
        || from.debug_assertions != new.debug_assertions
        || from.panic_strategy != new.panic_strategy
        || from.opt_level != new.opt_level
    {
        return Err("Forwarder and constructor come from different captured builds".into());
    }

    let from_census = from.frontend_options.data::<Arc<Census>>()?;
    let new_census = new.frontend_options.data::<Arc<Census>>()?;
    if from_census != new_census
        || from.frontend_options.get("rstyle.rust.compiled-crate")
            != new.frontend_options.get("rstyle.rust.compiled-crate")
        || from.frontend_options.get("invocation") != new.frontend_options.get("invocation")
    {
        return Err("Forwarder and constructor lack their exact shared source/build owner".into());
    }
    Ok(())
}
