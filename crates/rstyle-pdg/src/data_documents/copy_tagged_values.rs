//! Independently certify the exact original saved-value conversion graph.
use super::*;
use crate::{
    HirNode, nominal_boundary::resolution::CheckedCopyConversion, unit_store::EvidenceFacts,
};
pub(super) mod graph;
pub(crate) mod origins;
#[cfg(test)]
mod tests;

const CONSTRUCTS: [&str; 7] = [
    "data.enum-value",
    "match.enum-variant",
    "data.enum-payload",
    "match.wildcard",
    "data.result-ok",
    "data.result-err",
    "data.struct",
];

pub(crate) fn verify(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    receipt: &CheckedCopyConversion,
) -> Result<(), String> {
    let source = crate::copy_tagged_values::expression(facts)?;
    let expected = if receipt.error().is_none() {
        let tag = crate::copy_tagged_values::constructor(source, facts, receipt)?;
        let node = operation(pair, source, CONSTRUCTS[0], Some(&tag))?;
        let saved = value_operation(pair, &source.children[0])?;
        verify_saved_operands(pair, node, &[saved])?;
        graph::consumers(pair, saved, &[node], "value_operand")?;
        graph::returned(pair, source, node)?;
        [1, 0, 0, 0, 0, 0, 0]
    } else {
        inverse(pair, facts, receipt)?;
        [0, 1, 1, 1, 1, 1, 1]
    };

    verify_construct_inventory(pair, CONSTRUCTS, expected)?;
    // These complete functions contain one formal, the checked value operations,
    // and one return. No unrelated scalar/effect node may hide beside them.
    let node_count = if receipt.error().is_none() { 4 } else { 12 };
    if pair.pdg.nodes.len() != node_count {
        return Err("Copy conversion graph contains unaccounted operations".into());
    }

    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        crate::copy_tagged_values::CAPABILITY,
        || true,
        "Original Copy conversion graph requires its registered policy",
    )
}

fn inverse(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    receipt: &CheckedCopyConversion,
) -> Result<(), String> {
    let source = crate::copy_tagged_values::expression(facts)?;
    let checked = crate::copy_tagged_values::matched(source, facts, receipt)?;
    let saved = value_operation(pair, checked.scrutinee)?;
    let selector = operation(pair, checked.pattern, CONSTRUCTS[1], Some(&checked.tag))?;
    let payload = operation(pair, checked.binding, CONSTRUCTS[2], Some(&checked.tag))?;
    let wildcard = operation(pair, checked.wildcard, CONSTRUCTS[3], None)?;
    verify_saved_operands(pair, selector, &[saved])?;
    verify_saved_operands(pair, payload, &[saved])?;
    verify_saved_operands(pair, wildcard, &[])?;
    super::conditional_payloads::binding(pair, checked.binding, payload)?;
    super::conditional_payloads::verify(pair, selector, payload)?;
    error_descriptor(pair, facts, &checked)?;

    alternatives(pair, &checked, selector, wildcard)?;
    graph::inverse(pair, source, &checked)
}

fn error_descriptor(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    checked: &crate::copy_tagged_values::Match<'_>,
) -> Result<(), String> {
    let source = checked
        .failure
        .children
        .first()
        .ok_or("Missing original error value")?;
    let error = operation(pair, source, CONSTRUCTS[6], None)?;
    let expected = crate::data_structs::descriptor(source, facts)?;

    if error
        .attributes
        .literal
        .as_ref()
        .and_then(crate::StringFact::known_value)
        != Some(expected.as_str())
    {
        return Err("Copy conversion error loses its exact nominal unit descriptor".into());
    }
    Ok(())
}

fn alternatives(
    pair: PairView<'_>,
    checked: &crate::copy_tagged_values::Match<'_>,
    selector: &PdgNode,
    wildcard: &PdgNode,
) -> Result<(), String> {
    let ok = operation(pair, checked.success, CONSTRUCTS[4], None)?;
    let err = operation(pair, checked.failure, CONSTRUCTS[5], None)?;
    let error_source = checked
        .failure
        .children
        .first()
        .ok_or("Missing original error value")?;
    let error = operation(pair, error_source, CONSTRUCTS[6], None)?;

    verify_saved_operands(
        pair,
        ok,
        &[value_operation(pair, &checked.success.children[0])?],
    )?;
    verify_saved_operands(pair, err, &[error])?;
    verify_saved_operands(pair, error, &[])?;

    let guards = super::conditional_payloads::Guards::new(pair)?;

    guards.guarded(selector, ok, crate::CfgEdgeKind::True)?;
    guards.guarded(selector, wildcard, crate::CfgEdgeKind::False)?;
    guards.guarded(selector, error, crate::CfgEdgeKind::False)?;
    guards.guarded(selector, err, crate::CfgEdgeKind::False)
}

fn operation<'a>(
    pair: PairView<'a>,
    source: &HirNode,
    construct: &str,
    tag: Option<&str>,
) -> Result<&'a PdgNode, String> {
    let node = serialized_operation(pair, &source.span, |node| {
        node.attributes.construct.as_deref() == Some(construct)
    })?;
    let (category, ty) = if construct.starts_with("match.") {
        (NodeCategory::Control, "bool")
    } else {
        (
            NodeCategory::Computation,
            crate::data_aggregates::token(source)?,
        )
    };
    if node.category != category
        || node.original_kind.known_value() != Some(source.kind.as_str())
        || node
            .annotations
            .get("compat.rust.hir-r0:type")
            .and_then(|ty| ty.get("value"))
            .and_then(serde_json::Value::as_str)
            != Some(ty)
        || node
            .attributes
            .operator
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != tag
        || (construct != "data.struct" && node.attributes.literal.is_some())
        || node.attributes.callee.is_some()
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || node.attributes.write_form.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err(
            "Copy conversion operation loses its exact source, tag, type or effects".into(),
        );
    }
    Ok(node)
}
