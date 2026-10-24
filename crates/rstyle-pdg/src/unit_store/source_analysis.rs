//! Borrowed, immutable capture inputs; no helper certification or paired documents.
use super::{
    EvidenceFacts,
    derived::{Checked, ReferenceOrigins, Sources},
};
use crate::{HirFnFacts, data_receivers::Capture, nominal_boundary::resolution};
use std::borrow::Cow;

#[cfg(test)]
mod tests;

/// Checked lexical environments shared only by immutable census ownership and policy.
#[derive(Default)]
pub struct SourceAnalyses {
    sources: Sources,
}

impl SourceAnalyses {
    pub fn function<'a>(&'a self, facts: &'a HirFnFacts) -> SourceAnalysis<'a> {
        SourceAnalysis {
            facts,
            sources: &self.sources,
            nominal: Checked::default(),
            receivers: Default::default(),
            reference_origins: Default::default(),
        }
    }
}

/// Reuse source proofs while observing one exact immutable capture snapshot.
/// Drop this scope before modifying facts; strict and generated checks stay separate.
pub struct SourceAnalysis<'a> {
    facts: &'a HirFnFacts,
    sources: &'a Sources,
    nominal: Checked<resolution::Resolution>,
    receivers: [Checked<Capture>; 2],
    reference_origins: ReferenceOrigins,
}

impl super::sealed::Sealed for SourceAnalysis<'_> {}

impl EvidenceFacts for SourceAnalysis<'_> {
    fn facts(&self) -> &HirFnFacts {
        self.facts
    }

    fn verify_data_frontend(&self) -> Result<(), String> {
        // Keep nested source queries on this exact immutable analysis. Final
        // frontend/helper checks still run on every request.
        crate::data_frontend::check(self)
    }

    fn nominal_resolution(&self) -> Result<resolution::ResolutionView<'_>, String> {
        self.nominal
            .get(|| self.sources.resolve(self))
            .map(resolution::ResolutionView::borrowed)
    }

    fn receiver_inventory(&self, original_source: bool) -> Result<Cow<'_, Capture>, String> {
        super::derived::receivers(&self.receivers, self, original_source)
    }

    fn reference_origins(
        &self,
        node: &crate::HirNode,
        selectors: &[usize],
    ) -> Result<Vec<crate::data_origins::Origin>, String> {
        self.reference_origins.select(node, selectors, self)
    }
}
