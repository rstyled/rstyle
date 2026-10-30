//! Retain independently validated selected-unit context for immutable owner queries.
use super::Availability;
use crate::{
    HirFnFacts,
    nominal_boundary::{self, UnitContext},
};
use serde::Deserialize;
use std::collections::{BTreeMap, btree_map::Entry};

pub(super) fn retain(
    contexts: &mut BTreeMap<usize, Availability<UnitContext>>,
    functions: &[HirFnFacts],
    index: usize,
) -> Result<(), String> {
    if let Entry::Vacant(entry) = contexts.entry(index) {
        let facts = functions
            .get(index)
            .ok_or("Consumer function ordinal is outside its owning unit")?;
        let context = match facts
            .build
            .frontend_options
            .get(nominal_boundary::CONTEXT_KEY)
        {
            Some(value) => {
                let context: UnitContext = UnitContext::deserialize(value)
                    .map_err(|error| format!("Invalid consumer selected-unit context: {error}"))?;
                context.validate()?;
                Availability::Known(context)
            }
            None => Availability::Unknown {
                reason: "Consumer selected-unit context is absent".into(),
            },
        };
        entry.insert(context);
    }
    Ok(())
}
