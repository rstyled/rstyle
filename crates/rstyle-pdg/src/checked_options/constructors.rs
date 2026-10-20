//! Resolved scalar variants are value operations, never calls with invented purity.
use super::*;

pub(crate) fn constructor_candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    scalar_enabled(facts)
        && matches!(node.kind.as_str(), "expr_call" | "expr_path")
        && node
            .res
            .as_ref()
            .is_some_and(|res| res.kind == "resolved_option_variant")
}

pub(crate) fn verify_constructor(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<&'static str, String> {
    if !constructor_candidate(node, facts) || !owned(node) {
        return Err("Option construction requires its concrete scalar capability".into());
    }

    let scalar = types::payload(node)?;
    let (variant, construct) = if node.kind == "expr_call" {
        let [payload] = node.children.as_slice() else {
            return Err("Some requires exactly one evaluated scalar payload".into());
        };
        if crate::data_aggregates::token(payload)? != scalar {
            return Err("Some payload disagrees with its concrete Option type".into());
        }
        ("Some", "data.option-some")
    } else {
        if !node.children.is_empty() {
            return Err("None cannot manufacture a payload".into());
        }
        ("None", "data.option-none")
    };
    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing Option constructor identity")?;
    let callee: Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    if resolution.target.is_some() || node.expansion.is_some() || node.dispatch.is_some() {
        return Err("Option constructor requires direct unexpanded variant evidence".into());
    }
    authority(&callee, &format!("std::prelude::v1::{variant}"), facts)?;
    source::constructor(node, facts)?;
    Ok(construct)
}

pub(crate) fn constructors(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::field_locations::selected_operations(
        facts,
        &["expr_call", "expr_path"],
        constructor_candidate,
    )
}
