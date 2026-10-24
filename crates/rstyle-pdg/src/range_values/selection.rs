//! Bind the only supported trait call to its pinned trait and exact local body.
use crate::{HirNode, data_library::Callee, unit_store::EvidenceFacts};

pub(crate) fn verify(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
    resolution: &serde_json::Value,
) -> Result<(), String> {
    if resolution.get("trait_selection").is_none()
        && !(callee.name.rsplit("::").next() == Some("from")
            && callee.signature.return_ty.normalized.as_deref() == Some(super::TOKEN))
    {
        return Ok(());
    }

    let owner = crate::nominal_boundary::resolution::range_from_owner(callee)?;
    let selection = resolution.get("trait_selection");
    if owner.is_none() && selection.is_none() {
        return Ok(());
    }
    let (_, parameter) = owner.ok_or("Selected From call lacks its exact source implementation")?;
    if !crate::extraction_profile::facts_support(caller, super::CAPABILITY) {
        return Err("Selected Range From call requires its registered capability".into());
    }

    let observed: Callee =
        serde_json::from_value(selection.cloned().ok_or("Missing pinned From selection")?)
            .map_err(|e| e.to_string())?;
    trait_definition(&observed, caller)?;

    let implementation = observed
        .implementation
        .ok_or("Missing selected local From implementation")?;
    if implementation.implementation.is_some()
        || implementation.crate_hash != "local-unencoded"
        || implementation.path != callee.name
        || resolution["crate"].as_str() != Some(implementation.crate_name.as_str())
        || resolution["definition_identity"].as_str()
            != Some(implementation.definition_hash.as_str())
        || callee.signature.params.len() != 1
        || callee.signature.params[0].ty.normalized.as_deref()
            != Some(crate::data_types::token(&parameter)?.as_str())
        || !super::is_range(node)
    {
        return Err(
            "Range From implementation/input disagrees with exact linked source body".into(),
        );
    }

    crate::nominal_boundary::resolution::range_from_call(node, caller)
}

fn trait_definition(observed: &Callee, caller: &dyn EvidenceFacts) -> Result<(), String> {
    let registry = crate::data_library::registry_for(caller.facts())
        .ok_or("Unpinned Range From compiler/target")?;
    let mut root = observed.clone();
    root.implementation = None;
    if !["core::convert::From::from", "std::convert::From::from"].contains(&root.path.as_str())
        || !registry.exact_core_definition(&root, "std::convert::From::from")
    {
        return Err("Range From selection disagrees with reviewed core trait".into());
    }

    crate::data_library::verify_sysroot_roots(caller, &["core", "std"], "Range From")?;
    Ok(())
}
