//! The receiver's implicit Copy load is independent of the callee's field uses.
use super::*;
use crate::{HirNode, data_origins::Origin, unit_store::EvidenceFacts};

pub(super) fn locations(
    call: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Vec<(Origin, TypeToken)>, String> {
    if !crate::owned_receiver::shared_copy(call, facts)? {
        return Ok(Vec::new());
    }

    let receiver = call
        .children
        .first()
        .ok_or("Missing shared Copy receiver")?;
    let TypeToken::Shared(owner) =
        crate::data_types::parse(crate::data_aggregates::token(receiver)?)?
    else {
        return Err("Copy receiver lacks its shared source type".into());
    };
    let origins = crate::data_origins::origins(receiver, facts)?;
    if origins.is_empty() {
        return Err("Shared Copy receiver lacks saved formal origins".into());
    }
    if origins.len() > crate::concrete_types::MAX_ENTRIES {
        return Err(BUDGET_ERROR.into());
    }

    origins
        .into_iter()
        .map(|origin| {
            if type_at(facts, origin.parameter, &origin.path)? != *owner {
                return Err("Copy receiver origin contradicts its complete referent type".into());
            }
            Ok((origin, *owner.clone()))
        })
        .collect()
}
