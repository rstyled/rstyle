//! Bounded source and product authority for privately proved From instances.
use super::{Haystack, shared_bytes};
use crate::{data_origins::Origin, field_locations::Projection, unit_store::EvidenceFacts};

pub const CAPABILITY: &str = "rust.checked-reference-forwarder-instances";

pub fn verify(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = facts.reference_forwarder() else {
        return Ok(false);
    };
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || checked.constructor().haystack() == Haystack::Str
            && !crate::extraction_profile::facts_support(
                facts,
                super::activation::STRING_CAPABILITY,
            )
    {
        return Err("Checked From instance requires its registered capability".into());
    }
    if checked.facts() != facts.facts()
        || facts.source_scan_complete
        || !facts.source_diagnostics.is_empty()
    {
        return Err(
            "Checked From instance differs from its proved complete source/finding roster".into(),
        );
    }
    Ok(true)
}

pub fn verify_returned_origin(
    origin: &Origin,
    facts: &dyn EvidenceFacts,
    result: &crate::data_types::TypeToken,
) -> Result<(), String> {
    if !verify(facts)?
        || !facts
            .reference_forwarder()
            .is_some_and(|checked| checked.constructor().haystack() == Haystack::Str)
        || origin.parameter != 0
        || origin.path != [Projection::SharedReferent, Projection::Utf8Bytes]
        || *result != shared_bytes()
    {
        return Err("From string component lacks its exact checked formal/view/type proof".into());
    }
    Ok(())
}
