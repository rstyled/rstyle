//! Exact core variant authority and one saved, once-evaluated payload.
use super::*;
use crate::data_library::Callee;

/// Verified source operands and branch; no extraction or ownership permission.
pub struct Constructor<'a> {
    pub payload: &'a HirNode,
    pub construct: &'static str,
}

pub(crate) fn constructor_candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::gated_node(node, facts, CAPABILITY, |node| {
        node.kind == "expr_call" && node.res.as_ref().is_some_and(|res| res.kind == RESOLUTION)
    })
}

pub fn verify_constructor<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Constructor<'a>, String> {
    if !constructor_candidate(node, facts) || node.expansion.is_some() || node.dispatch.is_some() {
        return Err("Result construction requires its original direct core variant".into());
    }

    let [payload] = node.children.as_slice() else {
        return Err("Result construction requires exactly one evaluated payload".into());
    };

    let (ok, err) = types::payload(node, facts)?;
    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing Result variant resolution")?;
    if resolution.target.is_some() {
        return Err("Core Result variant cannot have a local binding target".into());
    }

    let callee: Callee =
        serde_json::from_str(&resolution.description).map_err(|e| e.to_string())?;
    let (variant, construct, expected) = match callee.path.rsplit("::").next() {
        Some("Ok") => ("Ok", "data.result-ok", ok),
        Some("Err") => ("Err", "data.result-err", err),
        _ => return Err("Unknown concrete Result variant".into()),
    };

    authority(&callee, variant, facts)?;
    if crate::data_aggregates::parsed_type(payload)? != expected {
        return Err("Result variant payload disagrees with its exact concrete branch".into());
    }

    source::constructor(node, facts, variant)?;
    Ok(Constructor { payload, construct })
}

fn authority(callee: &Callee, variant: &str, facts: &dyn EvidenceFacts) -> Result<(), String> {
    verify_core(facts)?;
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Unpinned concrete Result compiler/target")?;

    let expected = format!("std::prelude::v1::{variant}");
    let paths = [
        expected.clone(),
        format!("core::prelude::v1::{variant}"),
        format!("core::result::Result::{variant}"),
        format!("std::result::Result::{variant}"),
    ];

    if !paths.contains(&callee.path) || !registry.exact_core_definition(callee, &expected) {
        return Err("Result constructor lacks its exact pinned core definition/source".into());
    }
    Ok(())
}

pub(super) fn verify_core(facts: &dyn EvidenceFacts) -> Result<(), String> {
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Unpinned concrete Result compiler/target")?;
    if registry.source_digests.get("result.rs").map(String::as_str) != Some(SOURCE_DIGEST)
        || !registry.definitions.contains_key("std::result::Result")
    {
        return Err("Concrete Result lacks its reviewed core type/source authority".into());
    }
    crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "Concrete Result")
}

pub(crate) fn constructors(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::field_locations::selected_operations(facts, &["expr_call"], constructor_candidate)
}
