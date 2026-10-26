//! Shared source gates and tagged summaries retain each family's effect contract.
use super::{Summary, legacy_summary, tagged_conversions};
use crate::{copy_tagged_values::origins::WireContract, producer::Pair, unit_store::EvidenceFacts};

pub(super) struct Family {
    pub capability: &'static str,
    pub candidate: fn(&dyn EvidenceFacts) -> bool,
    pub source: fn(&dyn EvidenceFacts) -> Result<bool, String>,
    pub identity: &'static str,
    pub validate: fn(&Summary) -> Result<&WireContract, String>,
}

impl Family {
    pub(super) fn candidate(&self, facts: &dyn EvidenceFacts) -> bool {
        super::capability_candidate(facts, self.capability, self.candidate)
    }

    /// Source eligibility never substitutes for the family's complete graph replay.
    pub(super) fn supported(&self, facts: &dyn EvidenceFacts) -> Result<bool, String> {
        Ok(self.candidate(facts) && (self.source)(facts)?)
    }

    pub(super) fn summary(
        &self,
        facts: &dyn EvidenceFacts,
        pair: &Pair,
        output: WireContract,
    ) -> Result<Summary, String> {
        let mut summary = legacy_summary(facts, pair, None)?;
        summary.identity = self.identity.into();
        summary.result_dependence = tagged_conversions::RESULT_DEPENDENCE.into();
        summary.tagged_output = Some(output);

        (self.validate)(&summary)?;
        Ok(summary)
    }
}
