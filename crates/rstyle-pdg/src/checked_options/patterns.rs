//! Resolved variants and exact nested scalar binding identities.

use super::*;

fn variant<'a>(
    arm: &'a HirNode,
    name: &str,
    facts: &dyn EvidenceFacts,
) -> Result<&'a HirNode, String> {
    let [pattern, _] = arm.children.as_slice() else {
        return Err("Checked Option requires an unguarded pattern/body arm".into());
    };
    let expected = if name == "Some" {
        "pat_option_some"
    } else {
        "pat_option_none"
    };
    let resolution = pattern
        .res
        .as_ref()
        .filter(|res| res.kind == "resolved_option_variant" && res.target.is_none())
        .ok_or("Missing resolved core Option variant")?;
    let callee: Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;

    authority(&callee, &format!("std::prelude::v1::{name}"), facts)?;
    if pattern.kind != expected || pattern.expansion.is_some() {
        return Err("Checked Option pattern contradicts its variant".into());
    }
    Ok(pattern)
}

pub(super) fn none(arm: &HirNode, scalar: &str, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let pattern = variant(arm, "None", facts)?;
    if types::payload(pattern)? != scalar || !pattern.children.is_empty() {
        return Err("None cannot carry payload children".into());
    }
    Ok(())
}

pub(super) fn some<'a>(
    arm: &'a HirNode,
    scalar: &str,
    facts: &dyn EvidenceFacts,
) -> Result<&'a HirNode, String> {
    let pattern = variant(arm, "Some", facts)?;
    if types::payload(pattern)? != scalar {
        return Err("Option variant type disagrees with scrutinee".into());
    }
    let [shared] = pattern.children.as_slice() else {
        return Err("Some requires one exact reference pattern".into());
    };
    let [binding] = shared.children.as_slice() else {
        return Err("Some reference pattern requires one exact binding".into());
    };

    let referent = crate::data_types::TypeToken::Shared(Box::new(
        crate::data_types::TypeToken::Scalar(scalar.into()),
    ))
    .text()?;
    if shared.kind != "pat_shared"
        || crate::data_aggregates::token(shared)? != referent
        || binding.kind != "pat_binding"
        || crate::data_aggregates::token(binding)? != scalar
        || binding.binding_mut != Some(false)
        || !binding.children.is_empty()
        || shared.expansion.is_some()
        || binding.expansion.is_some()
    {
        return Err("Some requires an immutable scalar copy from a shared reference".into());
    }

    binding_identity(binding, facts)?;
    Ok(binding)
}

fn binding_identity(binding: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let candidates: Vec<_> = facts
        .bindings
        .iter()
        .filter(|fact| fact.hir == binding.hir)
        .collect();
    let [fact] = candidates.as_slice() else {
        return Err("Missing unique Option payload binding identity".into());
    };
    if binding.binding.as_deref() != Some(&fact.symbol)
        || fact.is_param
        || fact.is_mut
        || fact.decl_span != binding.span
        || fact.ty.normalized != binding.ty.as_ref().and_then(|ty| ty.normalized.clone())
    {
        return Err("Option payload binding contradicts captured binding inventory".into());
    }
    Ok(())
}
