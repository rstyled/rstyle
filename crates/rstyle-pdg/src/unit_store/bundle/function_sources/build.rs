//! Borrow build metadata while replacing only source-option wire records.
use super::*;
use crate::model::{BoolFact, CountFact, StringFact};

#[derive(Serialize)]
pub(in crate::unit_store::bundle) struct Export<'a> {
    frontend_options: option_bundle::Export<'a>,
    producer_revision: &'a StringFact,
    rustc_version: &'a StringFact,
    rustc_commit: &'a StringFact,
    edition: &'a StringFact,
    target_triple: &'a StringFact,
    pointer_width: &'a CountFact,
    data_layout: &'a StringFact,
    cfgs: &'a [String],
    features: &'a [String],
    overflow_checks: &'a BoolFact,
    debug_assertions: &'a BoolFact,
    panic_strategy: &'a StringFact,
    opt_level: &'a StringFact,
}
impl<'a> Export<'a> {
    pub fn new(
        facts: &'a BuildFacts,
        sources: &mut source_table::Owners<'a>,
        inventories: &mut option_bundle::Inventories<'a>,
    ) -> Result<Self, String> {
        Ok(Self {
            frontend_options: option_bundle::Export::new(
                &facts.frontend_options,
                sources,
                inventories,
            )?,
            producer_revision: &facts.producer_revision,
            rustc_version: &facts.rustc_version,
            rustc_commit: &facts.rustc_commit,
            edition: &facts.edition,
            target_triple: &facts.target_triple,
            pointer_width: &facts.pointer_width,
            data_layout: &facts.data_layout,
            cfgs: &facts.cfgs,
            features: &facts.features,
            overflow_checks: &facts.overflow_checks,
            debug_assertions: &facts.debug_assertions,
            panic_strategy: &facts.panic_strategy,
            opt_level: &facts.opt_level,
        })
    }
}
