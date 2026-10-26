//! Exact saved operands, consumers and CFG dominance precede origin publication.
pub(super) mod arithmetic;
use super::*;
use crate::named_error_payloads::Constructor;
use source::Checked;

pub(super) fn verify(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    checked: &Checked<'_>,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    if !pair.pdg.memory_locations.is_empty()
        || pair.cfg.extraction_evidence.status != crate::ExtractionStatus::Complete
    {
        return Err("Named errors require complete memory-free value evidence".into());
    }
    let expected = match checked {
        Checked::Constructor(function) => constructor(pair, facts, function)?,
        Checked::Root(root) => arithmetic::verify(pair, facts, root)?,
    };
    if expected.len() != pair.pdg.nodes.len() {
        return Err("Named error graph contains unaccounted or aliased operations".into());
    }
    Ok(expected)
}

fn constructor(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    function: &crate::named_error_payloads::Function<'_>,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    let source = &facts.body.children[0];
    let Constructor::Wrapper { payload, .. } = &function.wrapper else {
        unreachable!()
    };
    let Constructor::Named {
        variant, fields, ..
    } = &function.variant
    else {
        unreachable!()
    };
    let [field] = fields.as_slice() else {
        return Err("Incomplete named error fields".into());
    };

    let input = input(pair, &function.formal.pattern.span, 0)?;
    let read = value_operation(pair, field.operand)?;
    let named = operation(pair, payload, "data.named-error")?;
    let wrapper = operation(pair, source, "data.named-error-wrapper")?;

    let slots = serde_json::to_string(&[(field.position, field.name.as_str())])
        .map_err(|e| e.to_string())?;
    descriptor(named, Some(variant), &slots)?;
    descriptor(wrapper, None, "0")?;
    super::super::copy_tagged_values::origins::verify_definition(pair, input, read)?;

    edge(pair, read, named)?;
    edge(pair, named, wrapper)?;
    consumers(pair, input, &[read], "reaching_definition")?;

    consumers(pair, read, &[named], "value_operand")?;
    consumers(pair, named, &[wrapper], "value_operand")?;
    super::super::copy_tagged_values::graph::returned(pair, source, wrapper)?;

    let returned = returned(pair, source)?;
    let contract = crate::helper_calls::named_errors::source_contract(facts)?;
    origins::constructor([input, read, named, wrapper, returned], &contract)
}

fn descriptor(node: &PdgNode, tag: Option<&str>, literal: &str) -> Result<(), String> {
    if node
        .attributes
        .operator
        .as_ref()
        .and_then(crate::StringFact::known_value)
        != tag
        || node
            .attributes
            .literal
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some(literal)
    {
        return Err("Named error constructor changes its exact declaration descriptor".into());
    }
    Ok(())
}

fn operation<'a>(
    pair: PairView<'a>,
    source: &HirNode,
    construct: &str,
) -> Result<&'a PdgNode, String> {
    let node = serialized_operation(pair, &source.span, |node| {
        node.attributes.construct.as_deref() == Some(construct)
    })?;
    let ty = if construct.starts_with("match.") {
        "bool"
    } else {
        crate::data_aggregates::token(source)?
    };
    let category = if construct.starts_with("match.") {
        NodeCategory::Control
    } else if construct.starts_with("data.length.") {
        NodeCategory::Call
    } else {
        NodeCategory::Computation
    };

    if node.category != category
        || node.original_kind.known_value() != Some(source.kind.as_str())
        || node
            .annotations
            .get("compat.rust.hir-r0:type")
            .and_then(|ty| ty.get("value"))
            .and_then(serde_json::Value::as_str)
            != Some(ty)
        || (category != NodeCategory::Call && node.attributes.callee.is_some())
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || node.attributes.write_form.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Named error operation changes original type, source or effects".into());
    }
    Ok(node)
}

fn input<'a>(
    pair: PairView<'a>,
    span: &crate::ByteSpan,
    position: u64,
) -> Result<&'a PdgNode, String> {
    let input = serialized_operation(pair, span, |node| {
        node.category == NodeCategory::FormalInput
    })?;
    if input.attributes.position != Some(position) {
        return Err("Named error formal changes original position".into());
    }
    Ok(input)
}

fn returned<'a>(pair: PairView<'a>, source: &HirNode) -> Result<&'a PdgNode, String> {
    serialized_operation(pair, &source.span, |node| {
        node.category == NodeCategory::Return
    })
}

fn edge(pair: PairView<'_>, earlier: &PdgNode, later: &PdgNode) -> Result<(), String> {
    verify_saved_operands(pair, later, &[earlier])?;
    before(pair, earlier, later)
}

fn before(pair: PairView<'_>, earlier: &PdgNode, later: &PdgNode) -> Result<(), String> {
    super::super::callbacks::order::before(
        pair,
        &super::super::callbacks::order::completion(pair, earlier)?,
        later,
    )
}

use super::super::value_consumers::verify as consumers;
