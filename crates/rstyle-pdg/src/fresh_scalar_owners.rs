//! Fresh scalar/array owners require exact source proof and return-only use.
//! No Copy, local storage, helper transport or cleanup permission follows.
mod returns;
mod shape;
#[cfg(test)]
mod tests;

use crate::{data_types::TypeToken, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.fresh-scalar-owners";

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    checked_return(facts).is_ok_and(|owner| owner.is_some())
}

pub(crate) fn checked_return(facts: &dyn EvidenceFacts) -> Result<Option<TypeToken>, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Ok(None);
    }
    let Some(text) = facts.signature.return_ty.normalized.as_deref() else {
        return Ok(None);
    };

    let owner = crate::data_types::parse(text)?;
    if !shape::owner(&owner) {
        return Ok(None);
    }
    let resolution = facts.nominal_resolution()?;
    Ok(resolution.checked_fresh_owner(&owner).then_some(owner))
}

pub(crate) fn verify_function(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    match checked_return(facts)? {
        Some(owner) => returns::verify(facts, &owner).map(|()| true),
        None => Ok(false),
    }
}

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        CAPABILITY,
        || candidate(facts),
        "Fresh scalar owners require their registered document policy",
    )
}

/// Source cleanup evidence does not establish interprocedural ownership.
pub(crate) fn verify_callable(facts: &dyn EvidenceFacts) -> Result<(), String> {
    if verify_function(facts)? {
        return Err("Fresh owned results require a separately verified callable contract".into());
    }
    Ok(())
}
