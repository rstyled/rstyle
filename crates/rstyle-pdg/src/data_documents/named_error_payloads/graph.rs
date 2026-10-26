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
    let inputs = constructor_inputs(pair, function)?;
    let reads = fields
        .iter()
        .map(|field| value_operation(pair, field.operand))
        .collect::<Result<Vec<_>, _>>()?;
    let named = operation(pair, payload, "data.named-error")?;
    let wrapper = operation(pair, source, "data.named-error-wrapper")?;

    named_descriptor(named, variant, fields)?;
    descriptor(wrapper, None, "0")?;
    verify_fields(pair, fields, &inputs, &reads)?;
    verify_saved_operands(pair, named, &reads)?;
    constructor_consumers(pair, &reads, [named, wrapper])?;
    super::super::copy_tagged_values::graph::returned(pair, source, wrapper)?;

    let returned = returned(pair, source)?;
    let contract = crate::helper_calls::named_errors::source_contract(facts)?;
    if let Some(transfer) = crate::named_error_payloads::owned::checked(facts)? {
        return origins::owned_constructor(
            origins::OwnedNodes {
                error: [inputs[0], field_read(fields, &reads, 0)?],
                message: [inputs[1], field_read(fields, &reads, 1)?],
                outputs: [named, wrapper, returned],
            },
            &contract,
            &transfer,
        );
    }
    if function.formals.len() == 2 {
        return origins::scalar_constructor(&inputs, &reads, [named, wrapper, returned], &contract);
    }
    origins::constructor([inputs[0], reads[0], named, wrapper, returned], &contract)
}

fn constructor_consumers(
    pair: PairView<'_>,
    reads: &[&PdgNode],
    outputs: [&PdgNode; 2],
) -> Result<(), String> {
    let [named, wrapper] = outputs;
    for read in reads {
        before(pair, read, named)?;
        consumers(pair, read, &[named], "value_operand")?;
    }
    edge(pair, named, wrapper)?;
    consumers(pair, named, &[wrapper], "value_operand")
}

fn field_read<'a>(
    fields: &[crate::named_error_payloads::FieldOperand<'_>],
    reads: &[&'a PdgNode],
    parameter: u32,
) -> Result<&'a PdgNode, String> {
    let index = fields
        .iter()
        .position(|field| field.formal.position == parameter)
        .ok_or("Missing exact constructor operand")?;
    reads
        .get(index)
        .copied()
        .ok_or("Missing saved constructor value".into())
}

fn constructor_inputs<'a>(
    pair: PairView<'a>,
    function: &crate::named_error_payloads::Function<'_>,
) -> Result<Vec<&'a PdgNode>, String> {
    if function.formals.len() == 2 {
        crate::extraction_profile::require_document(
            &pair.pdg.profile,
            if matches!(
                function.formals[0].token,
                crate::data_types::TypeToken::Struct { .. }
            ) {
                crate::named_error_payloads::owned::CAPABILITY
            } else {
                crate::named_error_payloads::SCALAR_CAPABILITY
            },
            || true,
            "Scalar named error construction requires its registered document policy",
        )?;
    }
    function
        .formals
        .iter()
        .map(|formal| input(pair, &formal.pattern.span, u64::from(formal.position)))
        .collect()
}

fn named_descriptor(
    node: &PdgNode,
    variant: &str,
    fields: &[crate::named_error_payloads::FieldOperand<'_>],
) -> Result<(), String> {
    let slots: Vec<_> = fields
        .iter()
        .map(|field| (field.position, field.name.as_str()))
        .collect();
    descriptor(
        node,
        Some(variant),
        &serde_json::to_string(&slots).map_err(|error| error.to_string())?,
    )
}

fn verify_fields(
    pair: PairView<'_>,
    fields: &[crate::named_error_payloads::FieldOperand<'_>],
    inputs: &[&PdgNode],
    reads: &[&PdgNode],
) -> Result<(), String> {
    for (field, read) in fields.iter().zip(reads) {
        let input = inputs
            .get(field.formal.position as usize)
            .ok_or("Missing named error formal input")?;
        super::super::copy_tagged_values::origins::verify_definition(pair, input, read)?;
        consumers(pair, input, &[read], "reaching_definition")?;
    }
    for adjacent in reads.windows(2) {
        before(pair, adjacent[0], adjacent[1])?;
    }
    Ok(())
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

use super::super::value_consumers::verify as consumers;

use super::super::callbacks::order::completed_before as before;

pub(super) fn install_owned(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let expected = owned_annotations(pair.as_view(), facts)?;
    for node in &mut pair.pdg.nodes {
        if let Some(record) = expected.get(&node.id) {
            node.annotations.insert(
                crate::named_error_payloads::owned::KEY.into(),
                record.clone(),
            );
        }
    }
    Ok(())
}

pub(super) fn verify_owned(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let key = crate::named_error_payloads::owned::KEY;
    if pair.pdg.extensions.contains_key(key) || pair.cfg.extensions.contains_key(key) {
        return Err("Whole owned transfers require exact per-operation evidence".into());
    }
    let expected = owned_annotations(pair, facts)?;

    for node in &pair.pdg.nodes {
        if node.annotations.get(key) != expected.get(&node.id) {
            return Err(
                "Whole owned transfer annotation differs from original source and exact consumers"
                    .into(),
            );
        }
    }
    Ok(())
}

fn owned_annotations(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    let Some(transfer) = crate::named_error_payloads::owned::checked(facts)? else {
        return Ok(BTreeMap::new());
    };
    let function = crate::named_error_payloads::verify_function(facts)?;
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        crate::named_error_payloads::owned::CAPABILITY,
        || true,
        "Whole owned error transfer requires its registered document policy",
    )?;

    let Constructor::Wrapper { payload, .. } = &function.wrapper else {
        unreachable!()
    };
    let Constructor::Named { fields, .. } = &function.variant else {
        unreachable!()
    };
    let field = fields
        .iter()
        .find(|field| field.formal.position == transfer.parameter)
        .ok_or("Missing exact whole owned error field")?;
    let source = &facts.body.children[0];
    let operations = [
        input(pair, &function.formals[0].pattern.span, 0)?,
        value_operation(pair, field.operand)?,
        operation(pair, payload, "data.named-error")?,
        operation(pair, source, "data.named-error-wrapper")?,
        returned(pair, source)?,
    ];
    let record = serde_json::to_value(transfer).map_err(|error| error.to_string())?;
    let mut records = BTreeMap::new();
    for node in operations {
        if records.insert(node.id.clone(), record.clone()).is_some() {
            return Err("Whole owned transfer aliases its source or consuming operations".into());
        }
    }
    Ok(records)
}
