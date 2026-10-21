//! Independently check tagged construction, saved selectors and guarded payloads.
use super::*;
use crate::{HirNode, unit_store::EvidenceFacts};

const CONSTRUCTS: [&str; 3] = ["data.enum-value", "match.enum-variant", "data.enum-payload"];

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let constructors = crate::field_locations::selected_operations(
        facts,
        &["expr_call"],
        crate::data_enums::constructor_candidate,
    );
    let matches = crate::field_locations::selected_operations(
        facts,
        &["expr_match"],
        crate::data_enums::match_candidate,
    );
    let mut expected = [constructors.len(), 0, 0];
    for source in constructors {
        let tag = crate::data_enums::constructor(source, facts)?;
        let node = operation(pair, source, CONSTRUCTS[0], &tag)?;
        super::verify_saved_operands(
            pair,
            node,
            &[super::value_operation(pair, &source.children[0])?],
        )?;
    }
    for source in matches {
        let checked = crate::data_enums::matched(source, facts)?;
        expected[1] += checked.arms.len();
        for arm in &checked.arms {
            let selector = operation(pair, arm.pattern, CONSTRUCTS[1], &arm.tag)?;
            let saved = super::value_operation(pair, checked.scrutinee)?;
            super::verify_saved_operands(pair, selector, &[saved])?;
            if let Some(binding) = arm.binding {
                expected[2] += 1;
                let payload = operation(pair, binding, CONSTRUCTS[2], &arm.tag)?;
                super::verify_saved_operands(pair, payload, &[saved])?;
                super::conditional_payloads::binding(pair, binding, payload)?;
                super::conditional_payloads::verify(pair, selector, payload)?;
            }
        }
    }

    verify_discards(pair, facts)?;
    inventory(pair, expected)
}

fn inventory(pair: PairView<'_>, expected: [usize; 3]) -> Result<(), String> {
    super::verify_construct_inventory(pair, CONSTRUCTS, expected)?;
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        crate::data_enums::CAPABILITY,
        || expected.iter().any(|count| *count != 0),
        "Scalar enum document requires its registered policy",
    )
}

fn operation<'a>(
    pair: PairView<'a>,
    source: &HirNode,
    construct: &str,
    tag: &str,
) -> Result<&'a PdgNode, String> {
    let node = super::serialized_operation(pair, &source.span, |node| {
        node.attributes.construct.as_deref() == Some(construct)
    })?;
    let (category, ty) = if construct == "match.enum-variant" {
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
            != Some(tag)
        || node.attributes.literal.is_some()
        || node.attributes.callee.is_some()
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || node.attributes.write_form.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Scalar enum operation loses its exact tag, type, source or effects".into());
    }
    Ok(node)
}

fn verify_discards(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let sources = crate::field_locations::selected_operations(
        facts,
        &["stmt_let"],
        crate::data_discards::candidate,
    );
    if !sources.is_empty()
        && !crate::extraction_profile::policy_supports(
            &pair.pdg.profile,
            crate::data_enums::CAPABILITY,
        )
    {
        return Err("Copy discard requires its registered document policy".into());
    }

    super::verify_construct_inventory(pair, ["data.copy-discard"], [sources.len()])?;
    for source in sources {
        crate::data_discards::verify(source, facts)?;
        let node = super::serialized_operation(pair, &source.span, |node| {
            node.attributes.construct.as_deref() == Some("data.copy-discard")
        })?;
        if node.category != NodeCategory::Computation
            || node.original_kind.known_value() != Some("stmt_let")
            || !node.attributes.reads.is_empty()
            || !node.attributes.writes.is_empty()
            || !node.attributes.defines.is_empty()
            || !node.attributes.symbol_ids.is_empty()
            || node.attributes.operator.is_some()
            || node.attributes.literal.is_some()
            || node.attributes.callee.is_some()
        {
            return Err("Copy discard must perform no value read, write, call or binding".into());
        }
        super::verify_saved_operands(pair, node, &[])?;
    }
    Ok(())
}
