//! Exact reader success types; capture shapes never establish Copy or cleanup.
use crate::{
    concrete_types::{InterfaceObservations, Observation, TypeTree},
    data_types::TypeToken,
    unit_store::EvidenceFacts,
};

pub(crate) const CAPABILITY: &str = "rust.named-error-id-reader-flow";

pub(crate) fn shape(token: &TypeToken) -> bool {
    crate::concrete_types::payload_errors::id_reader(token)
}

/// Canonical Struct tokens erase generation. Keep both original declarations
/// explicit in the full tree before any observational or ownership selection.
pub(crate) fn product(success: &TypeTree) -> Option<&TypeTree> {
    if !shape(&TypeToken::from_tree(success)) {
        return None;
    }
    let TypeTree::Tuple(values) = success else {
        return None;
    };
    let [root, _] = values.as_slice() else {
        return None;
    };
    original_value(root)
}

pub(crate) fn original_value(tree: &TypeTree) -> Option<&TypeTree> {
    if !crate::concrete_types::payload_errors::id_value(&TypeToken::from_tree(tree)) {
        return None;
    }
    let TypeTree::Struct { fields, .. } = tree else {
        return None;
    };
    matches!(fields.as_slice(), [field] if matches!(field.ty, TypeTree::Struct { .. }))
        .then_some(tree)
}

/// An intermediate ID token must be the complete reader return's original ID.
pub(crate) fn value(token: &TypeToken, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    checked_return(token, facts, SuccessPart::Id)
}

pub(crate) fn success(token: &TypeToken, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    checked_return(token, facts, SuccessPart::Product)
}

/// Both uses share the captured reader boundary and its ownership proof, while
/// selecting different exact components for token comparison.
enum SuccessPart {
    Id,
    Product,
}

fn checked_return(
    token: &TypeToken,
    facts: &dyn EvidenceFacts,
    part: SuccessPart,
) -> Result<bool, String> {
    let applicable = match part {
        SuccessPart::Id => crate::concrete_types::payload_errors::id_value(token),
        SuccessPart::Product => shape(token),
    };
    if !applicable || !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Ok(false);
    }

    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    interface.validate()?;
    let Observation::Captured {
        ty: TypeTree::Result { ok, .. },
    } = &interface.result
    else {
        return Err("ID use requires its complete captured reader Result boundary".into());
    };
    let root = product(ok).ok_or("ID use requires its complete original reader return tuple")?;

    let expected = match part {
        SuccessPart::Id => root,
        SuccessPart::Product => ok.as_ref(),
    };
    if TypeToken::from_tree(expected) != *token {
        return Err("ID use differs from its exact complete reader return component".into());
    }
    id_success(root, facts)
}

/// Map input/output use the complete original tree, never its erased token alone.
pub(crate) fn id_success(tree: &TypeTree, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Ok(false);
    }
    let TypeTree::Struct { fields, .. } = tree else {
        return Ok(false);
    };
    let [field] = fields.as_slice() else {
        return Ok(false);
    };
    if !matches!(field.ty, TypeTree::Struct { .. })
        || !shape(&TypeToken::Tuple(vec![
            TypeToken::from_tree(tree),
            TypeToken::Scalar("usize".into()),
        ]))
    {
        return Ok(false);
    }

    crate::concrete_types::validate(tree)?;
    let resolution = facts.nominal_resolution()?;
    for dependency in [tree, &field.ty] {
        let nominal = dependency
            .nominal_identity()
            .ok_or("ID product lacks its original nominal")?;
        if !resolution.checked_owned_copy(dependency) || !resolution.checked_no_drop(nominal) {
            return Err(
                "ID reader requires independently checked complete Copy and cleanup".into(),
            );
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests;
