//! Four resolved core primitive instantiations with portable scalar semantics.

use crate::HirNode;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub const KEY: &str = "rstyle.hir-coverage:call-summaries";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub id: String,
    pub version: String,
    pub callee_path: String,
    pub callee_crate: String,
    pub argument_types: Vec<String>,
    pub result_type: String,
    pub memory_effects: bool,
    pub mutates_arguments: bool,
    pub may_panic: bool,
    pub always_returns: bool,
    pub overflow_behavior: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Callee {
    path: String,
    #[serde(rename = "crate")]
    crate_name: String,
    #[serde(default, rename = "crate_hash")]
    _crate_hash: String,
    #[serde(default, rename = "definition_hash")]
    _definition_hash: String,
}

pub fn registry() -> &'static [Summary] {
    static SUMMARIES: OnceLock<Vec<Summary>> = OnceLock::new();
    SUMMARIES.get_or_init(|| {
        serde_json::from_str(include_str!("../contracts/primitive-summaries-v1.json"))
            .expect("independently tested summary registry")
    })
}

pub fn resolve(node: &HirNode) -> Option<&'static Summary> {
    let dispatch = node.dispatch.as_ref()?;
    let resolution = node.res.as_ref()?;
    if node.kind != "expr_method_call"
        || dispatch.kind != "resolved_method"
        || resolution.kind != "resolved_callee"
        || resolution.target.is_some()
    {
        return None;
    }

    let callee: Callee = serde_json::from_str(&resolution.description).ok()?;
    // The compiler-resolved core callee and scalar signature identify the
    // operation. Build/core fingerprints explain captures, not these semantics.
    registry().iter().find(|summary| {
        summary.callee_path == dispatch.op
            && summary.callee_path == callee.path
            && summary.callee_crate == callee.crate_name
            && dispatch.operand_tys == summary.argument_types
            && dispatch.result_ty == summary.result_type
            && node.ty.as_ref().and_then(|ty| ty.normalized.as_ref()) == Some(&summary.result_type)
            && node.children.len() == summary.argument_types.len()
            && node
                .children
                .iter()
                .zip(&summary.argument_types)
                .all(|(child, expected)| {
                    child.ty.as_ref().and_then(|ty| ty.normalized.as_ref()) == Some(expected)
                })
    })
}

/// Profile-aware local authority; historical primitive resolution is unchanged.
pub fn resolve_for(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Option<Summary>, String> {
    if crate::size_calls::candidate(node, facts) {
        return crate::size_calls::resolve(node, facts);
    }
    if crate::bitset_cardinality::kind(node, facts).is_some() {
        return crate::bitset_cardinality::resolve(node, facts);
    }
    if crate::scalar_from::candidate(node, facts) {
        return crate::scalar_from::resolve(node, facts);
    }

    if let Some(summary) = resolve(node) {
        return Ok(Some(summary.clone()));
    }
    if crate::data_bytes::kind(node).is_some() && crate::data_bytes::enabled(facts) {
        return crate::data_bytes::resolve(node, facts);
    }
    if crate::checked_options::get_candidate(node, facts) {
        return crate::checked_options::resolve(node, facts).map(Some);
    }
    crate::data_library::resolve(node, facts)
}

pub(crate) fn verify_build(
    record: &serde_json::Value,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    for (key, expected) in [
        (
            "overflow_checks",
            serde_json::to_value(&facts.build.overflow_checks),
        ),
        (
            "debug_assertions",
            serde_json::to_value(&facts.build.debug_assertions),
        ),
        (
            "panic_strategy",
            serde_json::to_value(&facts.build.panic_strategy),
        ),
    ] {
        if record[key] != expected.map_err(|error| error.to_string())? {
            return Err("Serialized library call build settings disagree".into());
        }
    }

    Ok(())
}

pub(crate) fn static_intrinsic(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> bool {
    crate::bitset_cardinality::kind(node, facts).is_some()
        || crate::size_calls::candidate(node, facts)
        || crate::scalar_from::candidate(node, facts)
        || crate::data_bytes::enabled(facts) && crate::data_bytes::kind(node).is_some()
}

#[cfg(test)]
mod tests {
    #[test]
    fn code_owned_registry_preserves_frozen_primitive_contract_bytes() {
        assert_eq!(
            crate::concrete_types::digest(include_bytes!(
                "../contracts/primitive-summaries-v1.json"
            )),
            "sha256:d0b0b8a8f43f26584cedeff2e3abfab9ba1b5bd6e2c4304c00aa2cce11e42504"
        );
        assert_eq!(super::registry().len(), 4);
    }
}
