//! Fresh reference products returned directly in Result; no moves or helper export.
use super::*;
mod boundary;
mod returns;
mod shape;
#[cfg(test)]
mod tests;

pub(crate) use boundary::{ProductBoundary, product_boundary};
pub(crate) use returns::verify_product_returns;
pub(crate) use shape::{GRAMMAR, used};

pub(crate) const CAPABILITY: &str = "rust.fresh-reference-product-results";

/// Explicit checked Err path, including its transparent tail wrappers. A missing
/// origin set alone never establishes this exception.
pub(crate) fn empty_error_branch(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<bool, String> {
    let boundary = product_boundary(facts)?;
    empty_error_tail(node, &boundary.result, facts)
}

fn empty_error_tail(
    node: &HirNode,
    result: &TypeToken,
    facts: &dyn EvidenceFacts,
) -> Result<bool, String> {
    if node.kind == "expr_ret" {
        return match node.children.as_slice() {
            [value] => empty_error_tail(value, result, facts),
            _ => Ok(false),
        };
    }
    if crate::data_aggregates::parsed_type(node).as_ref() != Ok(result) {
        return Ok(false);
    }

    if node.kind == "expr_block" {
        return node
            .children
            .last()
            .map_or(Ok(false), |tail| empty_error_tail(tail, result, facts));
    }
    if !constructor_candidate(node, facts) {
        return Ok(false);
    }
    Ok(verify_constructor(node, facts)?.construct == "data.result-err")
}

pub(crate) fn interface_used(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    facts
        .signature
        .return_ty
        .normalized
        .as_deref()
        .map(crate::data_types::parse)
        .transpose()
        .map(|token| token.as_ref().is_some_and(used))
}

pub(super) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        CAPABILITY,
        || interface_used(facts).unwrap_or(false),
        "Fresh product Result requires its registered document capability",
    )
}
