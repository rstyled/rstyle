//! Bound materialized review owners, rather than unrelated captured functions.
use super::super::{ReviewStore, compiler_baseline};
use std::{cell::Cell, collections::BTreeSet, rc::Rc};

pub(in crate::clones) struct Demand<'a> {
    pub policy: Rc<serde_json::Value>,
    names: BTreeSet<&'a str>,
    remaining: Cell<usize>,
}

impl<'a> Demand<'a> {
    pub fn new(
        policy: Rc<serde_json::Value>,
        baseline: Option<&'a compiler_baseline::Baseline>,
        store: Option<&'a ReviewStore>,
    ) -> Self {
        let baseline = baseline.into_iter().flat_map(|baseline| &baseline.entries);
        let retained = store
            .into_iter()
            .flat_map(|store| &store.compiler_diagnostic_entries)
            .map(|entry| &entry.finding);
        Self {
            policy,
            names: baseline
                .chain(retained)
                .map(|finding| finding.owner.locator.qualified_name.as_str())
                .collect(),
            remaining: Cell::new(4096),
        }
    }

    pub fn includes(&self, facts: &rstyle_core::facts::FnFacts) -> bool {
        self.names
            .contains(rstyle_core::locator::base(facts).as_str())
    }

    pub fn claim(&self) -> bool {
        match self.remaining.get().checked_sub(1) {
            Some(remaining) => {
                self.remaining.set(remaining);
                true
            }
            None => false,
        }
    }
}
