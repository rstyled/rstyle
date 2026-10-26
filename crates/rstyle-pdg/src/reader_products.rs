//! Bounded aggregate reader results; observation alone grants no ownership.
use crate::{
    concrete_types::{InterfaceObservations, Observation, TypeTree},
    data_types::TypeToken,
    unit_store::EvidenceFacts,
};

pub(crate) const CAPABILITY: &str = "rust.named-error-product-reader-flow";

pub(crate) fn shape(token: &TypeToken) -> bool {
    crate::concrete_types::payload_errors::reader_product(token)
}

pub(crate) fn success(token: &TypeToken, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if !shape(token) || !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Ok(false);
    }

    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let Observation::Captured {
        ty: TypeTree::Result { ok, .. },
    } = &interface.result
    else {
        return Err("Product reader requires its complete captured Result boundary".into());
    };
    if TypeToken::from_tree(ok) != *token {
        return Err("Product reader success differs from its captured return boundary".into());
    }
    let TypeToken::Tuple(values) = token else {
        unreachable!()
    };
    let TypeToken::Struct { nominal, .. } = &values[0] else {
        unreachable!()
    };
    let resolution = facts.nominal_resolution()?;
    if !resolution.checked_copy_token(&values[0]) || !resolution.checked_no_drop(nominal) {
        return Err("Product reader lacks independent complete Copy and no-drop proof".into());
    }

    Ok(true)
}
