//! Saved values, exact consumers and real boolean branch transfers.
use super::*;
use crate::{CfgEdgeKind, HirNode};

pub(super) struct Operations<'a> {
    pub inputs: [&'a PdgNode; 3],
    pub receiver: &'a PdgNode,
    pub length: &'a PdgNode,
    pub minimum: &'a PdgNode,
    pub comparison: &'a PdgNode,
    pub selector: &'a PdgNode,
    pub message: &'a PdgNode,
    pub call: &'a PdgNode,
    pub failure: &'a PdgNode,
    pub early_return: &'a PdgNode,
    pub unit: &'a PdgNode,
    pub success: &'a PdgNode,
    pub returned: &'a PdgNode,
}

impl<'a> Operations<'a> {
    pub(super) fn all(&self) -> Vec<&'a PdgNode> {
        self.inputs
            .into_iter()
            .chain([
                self.receiver,
                self.length,
                self.minimum,
                self.comparison,
                self.selector,
                self.message,
                self.call,
                self.failure,
                self.early_return,
                self.unit,
                self.success,
                self.returned,
            ])
            .collect()
    }
}

pub(super) fn verify(
    pair: PairView<'_>,
    checked: &source::Checked<'_>,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    if !pair.pdg.memory_locations.is_empty()
        || pair.cfg.extraction_evidence.status != crate::ExtractionStatus::Complete
    {
        return Err("Slice length requires complete memory-free callee evidence".into());
    }
    let ops = operations(pair, checked)?;
    operands(pair, &ops)?;
    effects(checked, &ops)?;

    guards(pair, &ops)?;
    returns(pair, checked, &ops)?;
    let expected = origins::document(checked, &ops)?;

    let executable = ops
        .all()
        .into_iter()
        .filter(|node| node.category != NodeCategory::FormalInput)
        .map(|node| node.id.clone())
        .collect::<BTreeSet<_>>();
    if expected.len() != pair.pdg.nodes.len() || live_operations(pair)? != executable {
        return Err("Slice length contains omitted, duplicate or unreachable operations".into());
    }
    Ok(expected)
}

fn operations<'a>(
    pair: PairView<'a>,
    checked: &source::Checked<'_>,
) -> Result<Operations<'a>, String> {
    let instance = checked.instance;
    let inputs = instance
        .facts()
        .signature
        .params
        .iter()
        .map(|formal| {
            let input = serialized_operation(pair, &formal.pat.span, |node| {
                node.category == NodeCategory::FormalInput
            })?;
            if input.attributes.position != Some(u64::from(formal.position))
                || input.attributes.symbol_ids != [formal.symbol.clone()]
            {
                return Err("Slice length formal changes its exact binding position".into());
            }
            Ok(input)
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(Operations {
        inputs: inputs
            .try_into()
            .map_err(|_| "Slice length formal inventory changed")?,
        receiver: value_operation(pair, checked.receiver)?,
        length: value_operation(pair, instance.length_call())?,
        minimum: value_operation(pair, checked.minimum)?,
        comparison: value_operation(pair, instance.condition())?,
        selector: serialized_operation(pair, &instance.condition().span, |node| {
            node.category == NodeCategory::Control
                && node.attributes.construct.as_deref() == Some("if")
        })?,
        message: value_operation(pair, checked.message)?,
        call: value_operation(pair, instance.error_call())?,
        failure: value_operation(pair, instance.failure())?,
        early_return: returned(pair, instance.returned())?,
        unit: value_operation(pair, checked.unit)?,
        success: value_operation(pair, instance.success())?,
        returned: returned(pair, instance.success())?,
    })
}

fn operands(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    definitions(pair, ops)?;

    linear_operands(pair, ops)?;
    verify_saved_operands(pair, ops.comparison, &[ops.length, ops.minimum])?;
    for value in [ops.length, ops.minimum] {
        consumers(pair, value, &[ops.comparison], "value_operand")?;
    }

    before(pair, ops.length, ops.minimum)?;
    before(pair, ops.minimum, ops.comparison)?;
    for node in [ops.selector, ops.early_return, ops.returned] {
        consumers(pair, node, &[], "value_operand")?;
    }
    Ok(())
}

fn definitions(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for (input, read) in ops
        .inputs
        .into_iter()
        .zip([ops.receiver, ops.minimum, ops.message])
    {
        super::super::copy_tagged_values::origins::verify_definition(pair, input, read)?;
        consumers(pair, input, &[read], "reaching_definition")?;
    }
    Ok(())
}

fn linear_operands(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for (earlier, later) in [
        (ops.receiver, ops.length),
        (ops.comparison, ops.selector),
        (ops.message, ops.call),
        (ops.call, ops.failure),
        (ops.failure, ops.early_return),
        (ops.unit, ops.success),
        (ops.success, ops.returned),
    ] {
        verify_saved_operands(pair, later, &[earlier])?;
        consumers(pair, earlier, &[later], "value_operand")?;
        before(pair, earlier, later)?;
    }
    Ok(())
}

fn effects(checked: &source::Checked<'_>, ops: &Operations<'_>) -> Result<(), String> {
    symbolic_effects(checked, ops)?;
    for node in ops.all() {
        if !node.attributes.writes.is_empty()
            || node.attributes.write_form.is_some()
            || node.annotations.contains_key(crate::data_places::KEY)
        {
            return Err("Slice length invents memory or mutation effects".into());
        }
    }
    if ops.length.category != NodeCategory::Call
        || ops.call.category != NodeCategory::Call
        || ops.call.attributes.call_form != Some(crate::CallForm::Direct)
        || ops.comparison.category != NodeCategory::Computation
        || ops
            .comparison
            .attributes
            .operator
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some("<")
        || ops.comparison.original_kind.known_value() != Some("expr_binary")
        || ops.selector.original_kind.known_value() != Some("expr_binary")
    {
        return Err("Slice length changes its exact call, comparison or selector".into());
    }
    result_effects(checked, ops)
}

fn symbolic_effects(checked: &source::Checked<'_>, ops: &Operations<'_>) -> Result<(), String> {
    let reads = [ops.receiver, ops.minimum, ops.message];
    for ((input, read), formal) in ops
        .inputs
        .into_iter()
        .zip(reads)
        .zip(&checked.instance.facts().signature.params)
    {
        symbolic_binding(input, read, &formal.symbol)?;
    }
    let definitions = ops
        .inputs
        .iter()
        .flat_map(|node| &node.attributes.defines)
        .collect::<BTreeSet<_>>();
    if definitions.len() != 3 {
        return Err("Slice length formals share or omit a reaching definition".into());
    }

    for node in ops.all().into_iter().filter(|node| {
        !ops.inputs
            .iter()
            .chain(&reads)
            .any(|binding| binding.id == node.id)
    }) {
        if !node.attributes.symbol_ids.is_empty()
            || !node.attributes.defines.is_empty()
            || !node.attributes.reads.is_empty()
        {
            return Err(format!(
                "Slice length operation {} invents symbolic or memory effects",
                node.id
            ));
        }
    }
    Ok(())
}

fn symbolic_binding(input: &PdgNode, read: &PdgNode, symbol: &str) -> Result<(), String> {
    let expected = crate::Access {
        symbol_id: Some(symbol.into()),
        location_id: None,
        unresolved_reason: None,
        operand_position: Some(0),
    };
    if input.category != NodeCategory::FormalInput
        || input.attributes.symbol_ids != [symbol]
        || !matches!(input.attributes.defines.as_slice(), [definition] if !definition.is_empty())
        || !input.attributes.reads.is_empty()
        || read.category != NodeCategory::Read
        || read.attributes.symbol_ids != [symbol]
        || !read.attributes.defines.is_empty()
        || read.attributes.reads != [expected]
    {
        return Err(
            "Slice length changes an exact formal definition or saved symbolic read".into(),
        );
    }
    Ok(())
}

fn result_effects(checked: &source::Checked<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for (node, construct) in [
        (ops.failure, "data.result-err"),
        (ops.success, "data.result-ok"),
    ] {
        if node.category != NodeCategory::Computation
            || node.attributes.construct.as_deref() != Some(construct)
            || node
                .annotations
                .get("compat.rust.hir-r0:type")
                .and_then(|value| value.get("value"))
                .and_then(serde_json::Value::as_str)
                != checked
                    .instance
                    .facts()
                    .signature
                    .return_ty
                    .normalized
                    .as_deref()
        {
            return Err("Slice length changes its complete Result operation".into());
        }
    }
    Ok(())
}

fn guards(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    let guards = super::super::conditional_payloads::Guards::new(pair)?;
    for node in [ops.message, ops.call, ops.failure, ops.early_return] {
        guards.guarded(ops.selector, node, CfgEdgeKind::True)?;
    }
    for node in [ops.unit, ops.success, ops.returned] {
        guards.guarded(ops.selector, node, CfgEdgeKind::False)?;
    }
    Ok(())
}

fn returns(
    pair: PairView<'_>,
    checked: &source::Checked<'_>,
    ops: &Operations<'_>,
) -> Result<(), String> {
    for (node, form) in [(ops.early_return, "explicit"), (ops.returned, "implicit")] {
        if node
            .attributes
            .extensions
            .get("compat.rust.hir-r0:return-form")
            .and_then(serde_json::Value::as_str)
            != Some(form)
            || node
                .annotations
                .get("compat.rust.hir-r0:type")
                .and_then(|value| value.get("value"))
                .and_then(serde_json::Value::as_str)
                != checked
                    .instance
                    .facts()
                    .signature
                    .return_ty
                    .normalized
                    .as_deref()
        {
            return Err("Slice length loses its exact whole Result return".into());
        }
        super::super::result_propagation::return_to_exit(pair, node)?;
    }
    Ok(())
}

fn returned<'a>(pair: PairView<'a>, source: &HirNode) -> Result<&'a PdgNode, String> {
    serialized_operation(pair, &source.span, |node| {
        node.category == NodeCategory::Return
    })
}

use super::super::value_consumers::verify as consumers;

use super::super::callbacks::order::completed_before as before;
