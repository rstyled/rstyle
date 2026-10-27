//! Source extensions are checked before any shared environment can be reused.
use super::Census;
use crate::unit_store::EvidenceFacts;
use std::sync::Arc;

pub(crate) fn authority(facts: &dyn EvidenceFacts) -> Result<(), String> {
    let census = facts.build.frontend_options.data::<Arc<Census>>()?;
    super::documentation::authority(facts, &census)?;
    super::external_imports::authority(facts, &census)
}

pub(crate) fn verify_documents(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    if facts.build.frontend_options.get(super::KEY).is_none() {
        return Ok(());
    }
    let census = facts.build.frontend_options.data::<Arc<Census>>()?;
    for (capability, present) in [
        (
            super::documentation::CAPABILITY,
            !census.documentation.is_empty(),
        ),
        (
            super::external_imports::CAPABILITY,
            !census.external_imports.is_empty(),
        ),
    ] {
        if present && !crate::extraction_profile::policy_supports(profile, capability) {
            return Err(format!(
                "Source input requires its registered document capability: {capability}"
            ));
        }
    }

    if !census.external_imports.is_empty() {
        facts.verify_source_inputs()?;
    }
    Ok(())
}
