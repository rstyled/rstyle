//! Only a retained complete checked token activates this bounded instance family.
use super::{CAPABILITY, CheckedProjection};
use crate::unit_store::EvidenceFacts;

pub(crate) fn verify(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = facts.slice_length_instance() else {
        return Ok(false);
    };
    owning(facts, checked)?;
    Ok(true)
}

fn owning(facts: &dyn EvidenceFacts, checked: &CheckedProjection) -> Result<(), String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || checked.facts() != facts.facts()
    {
        return Err(
            "Slice instance activation lacks its exact owning checked facts/capability".into(),
        );
    }
    super::findings(facts.facts(), &checked.length_call().hir)
}
