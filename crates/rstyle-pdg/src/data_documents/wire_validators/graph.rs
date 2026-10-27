//! Saved scalar components, complete consumers, conditional transfers and effects.
use super::*;
use crate::CfgEdgeKind;

pub(super) struct Operations<'a> {
    pub inputs: Vec<&'a PdgNode>,
    pub reader: Reader<'a>,
    pub bindings: Bindings<'a>,
    pub assertion: Assertion<'a>,
    pub branch: Branch<'a>,
    pub tail: Tail<'a>,
}

pub(in crate::data_documents) struct Reader<'a> {
    pub slice: &'a PdgNode,
    pub message: &'a PdgNode,
    pub call: &'a PdgNode,
    pub selector: &'a PdgNode,
    pub error: &'a PdgNode,
    pub returned: &'a PdgNode,
    pub tuple: &'a PdgNode,
}
pub(super) struct Bindings<'a> {
    pub components: Vec<&'a PdgNode>,
    pub declarations: Vec<&'a PdgNode>,
}
pub(super) struct Assertion<'a> {
    pub read: &'a PdgNode,
    pub size: &'a PdgNode,
    pub call: &'a PdgNode,
}
pub(super) struct Branch<'a> {
    pub compared: [&'a PdgNode; 2],
    pub comparison: &'a PdgNode,
    pub predicate: &'a PdgNode,
    pub arguments: [&'a PdgNode; 2],
    pub call: &'a PdgNode,
    pub failure: &'a PdgNode,
    pub returned: &'a PdgNode,
}
pub(super) struct Tail<'a> {
    pub read: &'a PdgNode,
    pub success: &'a PdgNode,
    pub returned: &'a PdgNode,
}

pub(super) fn verify(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    checked: &source::Checked<'_>,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    let ops = operations(pair, facts, checked)?;
    effects(pair, &ops)?;
    literal(ops.reader.message, &checked.reader.literal)?;
    scalar_semantics(&ops)?;
    super::super::named_error_result_propagation::verify(pair, facts)?;
    super::super::scalar_tuple_bindings::verify(pair, facts)?;
    super::super::assertions::verify(pair, facts)?;
    crate::helper_calls::verify_documents(pair, facts)?;
    operands(pair, &ops)?;
    bindings(pair, &ops)?;
    tuple_consumers(pair, &ops)?;
    guarded(pair, &ops)?;

    let expected = origins::derive(facts, checked, &ops)?;
    if expected.len() != pair.pdg.nodes.len()
        || pair
            .pdg
            .nodes
            .iter()
            .any(|node| !expected.contains_key(&node.id))
    {
        return Err(
            "Wire validator contains omitted, duplicate or unaccounted graph operations".into(),
        );
    }
    Ok(expected)
}

fn operations<'a>(
    pair: PairView<'a>,
    facts: &dyn EvidenceFacts,
    checked: &source::Checked<'_>,
) -> Result<Operations<'a>, String> {
    let inputs = facts
        .signature
        .params
        .iter()
        .map(|parameter| {
            let node = serialized_operation(pair, &parameter.pat.span, |node| {
                node.category == NodeCategory::FormalInput
            })?;
            if node.attributes.position != Some(u64::from(parameter.position)) {
                return Err("Wire input changes original position".into());
            }
            Ok(node)
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Operations {
        inputs,
        reader: reader_operations(pair, &checked.reader)?,
        bindings: tuple_operations(pair, checked)?,
        assertion: Assertion {
            read: value_operation(pair, checked.assertion.read)?,
            size: value_operation(pair, checked.assertion.size)?,
            call: value_operation(pair, checked.assertion.node)?,
        },
        branch: branch_operations(pair, &checked.branch)?,
        tail: Tail {
            read: value_operation(pair, checked.success_read)?,
            success: value_operation(pair, checked.success)?,
            returned: returned(pair, checked.success)?,
        },
    })
}

pub(in crate::data_documents) fn reader_operations<'a>(
    pair: PairView<'a>,
    checked: &source::Reader<'_>,
) -> Result<Reader<'a>, String> {
    let propagation = &checked.propagation;
    let constructs = propagation.mode.constructs();
    Ok(Reader {
        slice: value_operation(pair, checked.slice)?,
        message: value_operation(pair, checked.message)?,
        call: value_operation(pair, propagation.operand)?,
        selector: typed_operation(pair, checked.question, constructs[0], "bool")?,
        error: typed_operation(
            pair,
            checked.question,
            constructs[1],
            &propagation.output_token,
        )?,
        returned: returned(pair, propagation.early_return)?,
        tuple: typed_operation(
            pair,
            checked.question,
            constructs[2],
            &propagation.payload_token,
        )?,
    })
}

fn branch_operations<'a>(
    pair: PairView<'a>,
    checked: &source::Branch<'_>,
) -> Result<Branch<'a>, String> {
    Ok(Branch {
        compared: [
            value_operation(pair, checked.compared[0])?,
            value_operation(pair, checked.compared[1])?,
        ],
        comparison: value_operation(pair, checked.condition)?,
        predicate: typed_operation(pair, checked.condition, "if", "bool")?,
        arguments: [
            value_operation(pair, checked.arguments[0])?,
            value_operation(pair, checked.arguments[1])?,
        ],
        call: value_operation(pair, checked.mismatch)?,
        failure: value_operation(pair, checked.failure)?,
        returned: returned(pair, checked.returned)?,
    })
}

fn tuple_operations<'a>(
    pair: PairView<'a>,
    checked: &source::Checked<'_>,
) -> Result<Bindings<'a>, String> {
    let components = checked
        .binding
        .entries
        .iter()
        .map(|entry| {
            typed_operation(
                pair,
                entry.pattern,
                crate::scalar_tuple_bindings::SELECTOR,
                entry.token,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let declarations = checked
        .binding
        .entries
        .iter()
        .map(|entry| {
            typed_operation(
                pair,
                entry.pattern,
                crate::scalar_tuple_bindings::DECLARATION,
                "unit",
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Bindings {
        components,
        declarations,
    })
}

fn returned<'a>(pair: PairView<'a>, source: &HirNode) -> Result<&'a PdgNode, String> {
    serialized_operation(pair, &source.span, |node| {
        node.category == NodeCategory::Return
    })
}

fn effects(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    if pair.cfg.extraction_evidence.status != crate::ExtractionStatus::Partial {
        return Err(
            "Wire validator must preserve its actual native-read and assertion panic boundaries"
                .into(),
        );
    }
    for node in &pair.pdg.nodes {
        if node.annotations.contains_key(crate::data_places::KEY)
            || !ops
                .bindings
                .declarations
                .iter()
                .any(|declaration| declaration.id == node.id)
                && (!node.attributes.writes.is_empty() || node.attributes.write_form.is_some())
            || node.id != ops.reader.call.id
                && node
                    .attributes
                    .reads
                    .iter()
                    .any(|read| read.location_id.is_some())
        {
            return Err("Wire validator invents writes, mutable places or reads outside its selected reader".into());
        }
    }
    Ok(())
}

fn operands(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for (node, values) in [
        (ops.reader.call, vec![ops.reader.slice, ops.reader.message]),
        (
            ops.assertion.call,
            vec![ops.assertion.read, ops.assertion.size],
        ),
        (ops.branch.comparison, ops.branch.compared.to_vec()),
        (ops.branch.predicate, vec![ops.branch.comparison]),
        (ops.branch.call, ops.branch.arguments.to_vec()),
        (ops.branch.failure, vec![ops.branch.call]),
        (ops.branch.returned, vec![ops.branch.failure]),
        (ops.tail.success, vec![ops.tail.read]),
        (ops.tail.returned, vec![ops.tail.success]),
    ] {
        verify_saved_operands(pair, node, &values)?;
        for value in values {
            super::super::value_consumers::verify(pair, value, &[node], "value_operand")?;
        }
    }
    Ok(())
}

fn bindings(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    let mut definitions = vec![
        (ops.inputs[0], vec![ops.reader.slice]),
        (
            ops.bindings.declarations[0],
            vec![ops.branch.compared[0], ops.branch.arguments[1]],
        ),
        (
            ops.bindings.declarations[1],
            vec![ops.assertion.read, ops.tail.read],
        ),
    ];
    if ops.inputs.len() == 2 {
        definitions.push((
            ops.inputs[1],
            vec![ops.branch.compared[1], ops.branch.arguments[0]],
        ));
    }
    for (definition, reads) in definitions {
        for read in &reads {
            super::super::copy_tagged_values::origins::verify_definition(pair, definition, read)?;
        }
        super::super::value_consumers::verify(pair, definition, &reads, "reaching_definition")?;
    }
    Ok(())
}

fn guarded(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    let guards = super::super::conditional_payloads::Guards::new(pair)?;
    reader_success(&guards, ops)?;
    mismatch_outcomes(&guards, ops)?;
    sequences(pair, ops)?;

    for returned in [ops.reader.returned, ops.branch.returned, ops.tail.returned] {
        super::super::result_propagation::return_to_exit(pair, returned)?;
    }
    Ok(())
}

fn reader_success(
    guards: &super::super::conditional_payloads::Guards<'_>,
    ops: &Operations<'_>,
) -> Result<(), String> {
    for node in ops
        .bindings
        .components
        .iter()
        .chain(&ops.bindings.declarations)
        .copied()
        .chain([
            ops.assertion.read,
            ops.assertion.size,
            ops.assertion.call,
            ops.branch.compared[0],
            ops.branch.compared[1],
            ops.branch.comparison,
            ops.branch.predicate,
            ops.branch.arguments[0],
            ops.branch.arguments[1],
            ops.branch.call,
            ops.branch.failure,
            ops.branch.returned,
            ops.tail.read,
            ops.tail.success,
            ops.tail.returned,
        ])
    {
        guards.guarded(ops.reader.selector, node, CfgEdgeKind::False)?;
    }
    Ok(())
}

fn mismatch_outcomes(
    guards: &super::super::conditional_payloads::Guards<'_>,
    ops: &Operations<'_>,
) -> Result<(), String> {
    for node in [
        ops.branch.arguments[0],
        ops.branch.arguments[1],
        ops.branch.call,
        ops.branch.failure,
        ops.branch.returned,
    ] {
        guards.guarded(ops.branch.predicate, node, CfgEdgeKind::True)?;
    }

    for node in [ops.tail.read, ops.tail.success, ops.tail.returned] {
        guards.guarded(ops.branch.predicate, node, CfgEdgeKind::False)?;
    }
    Ok(())
}

fn sequences(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for sequence in [
        vec![
            ops.reader.slice,
            ops.reader.message,
            ops.reader.call,
            ops.reader.selector,
        ],
        vec![
            ops.reader.tuple,
            ops.bindings.components[0],
            ops.bindings.declarations[0],
            ops.bindings.components[1],
            ops.bindings.declarations[1],
            ops.assertion.read,
            ops.assertion.size,
            ops.assertion.call,
            ops.branch.compared[0],
            ops.branch.compared[1],
            ops.branch.comparison,
            ops.branch.predicate,
        ],
        vec![
            ops.branch.arguments[0],
            ops.branch.arguments[1],
            ops.branch.call,
            ops.branch.failure,
            ops.branch.returned,
        ],
        vec![ops.tail.read, ops.tail.success, ops.tail.returned],
    ] {
        super::super::callbacks::order::sequence(pair, &sequence)?;
    }
    Ok(())
}

pub(in crate::data_documents) fn literal(
    node: &PdgNode,
    descriptor: &crate::static_string_values::Descriptor,
) -> Result<(), String> {
    super::super::verify_value_effects(node, true, "Wire static literal invents effects")?;
    if node.category != NodeCategory::Computation
        || node.original_kind.known_value() != Some("expr_lit")
        || node
            .attributes
            .literal
            .as_ref()
            .and_then(|value| value.known_value())
            != Some(descriptor.spelling.as_str())
        || node.attributes.write_form.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Wire static literal changes its exact source-backed saved value".into());
    }
    Ok(())
}

fn tuple_consumers(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    super::super::value_consumers::verify(
        pair,
        ops.reader.tuple,
        &ops.bindings.components,
        "value_operand",
    )?;
    for (selected, declared) in ops
        .bindings
        .components
        .iter()
        .zip(&ops.bindings.declarations)
    {
        super::super::value_consumers::verify(pair, selected, &[declared], "value_operand")?;
    }
    for terminal in [
        ops.reader.returned,
        ops.branch.returned,
        ops.tail.returned,
        ops.branch.predicate,
    ] {
        super::super::value_consumers::verify(pair, terminal, &[], "value_operand")?;
    }
    Ok(())
}

fn scalar_semantics(ops: &Operations<'_>) -> Result<(), String> {
    let comparison = ops.branch.comparison;
    if comparison.category != NodeCategory::Computation
        || comparison
            .attributes
            .operator
            .as_ref()
            .and_then(|value| value.known_value())
            != Some("!=")
        || comparison.attributes.literal.is_some()
        || comparison.attributes.callee.is_some()
        || comparison.attributes.call_form.is_some()
        || !comparison.attributes.reads.is_empty()
        || !comparison.attributes.writes.is_empty()
        || !comparison.attributes.defines.is_empty()
        || !comparison.attributes.symbol_ids.is_empty()
        || comparison.attributes.write_form.is_some()
    {
        return Err("Wire mismatch comparison changes builtin inequality or value effects".into());
    }

    let predicate = ops.branch.predicate;
    super::super::verify_value_effects(
        predicate,
        false,
        "Wire mismatch predicate invents effects",
    )?;
    if predicate.category != NodeCategory::Control || predicate.attributes.write_form.is_some() {
        return Err("Wire mismatch loses its exact boolean branch operation".into());
    }
    Ok(())
}
