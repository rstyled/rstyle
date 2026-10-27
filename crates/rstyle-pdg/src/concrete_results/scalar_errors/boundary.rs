//! Exact interface authority; no constructors, body certification or Copy errors.
use super::*;
use crate::concrete_types::{InterfaceObservations, Observation, TypeTree};

use super::super::interface::{self, Contract, ObservedResult};

pub(crate) type Boundary = interface::CheckedResult<()>;

pub(crate) fn checked_error(token: &TypeToken, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let nominal = error(token).ok_or("Fresh scalar error requires complete named scalar fields")?;
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Fresh scalar error requires its registered capability".into());
    }

    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    captured_error(&interface.result, token)?;
    if !facts.nominal_resolution()?.checked_no_drop(nominal) {
        return Err("Fresh scalar error lacks independent original no-drop proof".into());
    }
    Ok(())
}

/// Detection only: ordinary Copy products do not become non-Copy error values.
/// The complete fresh boundary still checks captured identity and capabilities.
pub(crate) fn owned_error(token: &TypeToken, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(nominal) = error(token) else {
        return Ok(false);
    };
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let Observation::Captured {
        ty: TypeTree::Result { err, .. },
    } = &interface.result
    else {
        return Ok(false);
    };
    if TypeToken::from_tree(err) != *token {
        return Ok(false);
    }

    let resolution = facts.nominal_resolution()?;
    Ok(resolution.checked_no_drop(nominal) && !resolution.checked_copy_token(token))
}

pub(super) fn captured_error(observation: &Observation, token: &TypeToken) -> Result<(), String> {
    let Observation::Captured { ty } = observation else {
        return Err("Fresh scalar error requires complete captured return evidence".into());
    };
    let TypeTree::Result { err, .. } = ty else {
        return Err("Fresh scalar error requires an established Result error role".into());
    };
    let error_tree = err.as_ref();

    if TypeToken::from_tree(error_tree) != *token {
        return Err("Fresh scalar error differs from its complete captured return".into());
    }
    Ok(())
}

pub(crate) fn result_boundary(facts: &dyn EvidenceFacts) -> Result<Boundary, String> {
    interface::verify(
        facts,
        Contract {
            capability: CAPABILITY,
            grammar: &GRAMMAR,
            check_error: checked_error,
            check_success: copy_success,
        },
    )
}

fn copy_success(
    observed: &ObservedResult<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<Option<()>, String> {
    Ok(facts
        .nominal_resolution()?
        .checked_copy_token(&observed.ok)
        .then_some(()))
}
