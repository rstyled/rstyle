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
        serde_json::from_str(include_str!(
            "../../../docs/extraction-profiles/primitive-summaries-v1.json"
        ))
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
pub fn resolve_for(node: &HirNode, facts: &crate::HirFnFacts) -> Result<Option<Summary>, String> {
    if let Some(summary) = resolve(node) {
        return Ok(Some(summary.clone()));
    }
    crate::data_library::resolve(node, facts)
}
