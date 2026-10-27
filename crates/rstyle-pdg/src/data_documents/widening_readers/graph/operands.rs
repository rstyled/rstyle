//! Exact saved operands and reaching definitions prevent capture/component rebinding.
use super::*;

pub(super) fn verify(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    checked: &contract::Checked<'_>,
    ops: &Operations<'_>,
) -> Result<(), String> {
    values(pair, ops)?;
    bindings(pair, ops)?;
    conversion(pair, facts, checked, ops)
}

fn values(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for (name, inputs) in [
        ("reader", vec!["slice", "message"]),
        ("outer-is-ok", vec!["reader"]),
        ("reader-ok-payload", vec!["reader"]),
        ("decoded-payload", vec!["reader-ok-payload"]),
        ("count-payload", vec!["reader-ok-payload"]),
        ("reader-error-forward", vec!["reader"]),
        ("conversion", vec!["decoded-read"]),
        ("map-is-ok", vec!["conversion"]),
        ("conversion-ok-payload", vec!["conversion"]),
        ("count-capture", vec!["map-environment"]),
        ("success-tuple", vec!["converted-read", "count-capture"]),
        ("map-ok", vec!["success-tuple"]),
        ("map-error-forward", vec!["conversion"]),
        ("failure-environment", vec!["outer-environment"]),
        ("failure-is-err", vec!["map-merge"]),
        ("conversion-error-discard", vec!["map-merge"]),
        ("message-capture", vec!["failure-environment"]),
        ("constructor", vec!["message-capture"]),
        ("final-error", vec!["constructor"]),
        ("final-success-payload", vec!["map-merge"]),
        ("final-ok", vec!["final-success-payload"]),
        ("returned", vec!["outer-merge"]),
        ("map-merge", vec![]),
        ("failure-merge", vec![]),
        ("outer-merge", vec![]),
    ] {
        let inputs = inputs
            .into_iter()
            .map(|name| ops.get(name))
            .collect::<Result<Vec<_>, _>>()?;
        crate::data_documents::verify_saved_operands(pair, ops.get(name)?, &inputs)?;
    }
    Ok(())
}

fn bindings(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for (binding, payload, read) in [
        (ops.bindings[0], "decoded-payload", "decoded-read"),
        (ops.bindings[1], "count-payload", "map-environment"),
        (ops.bindings[2], "conversion-ok-payload", "converted-read"),
    ] {
        crate::data_documents::verify_saved_operands(pair, binding, &[ops.get(payload)?])?;
        definition(pair, binding, &[ops.get(read)?])?;
    }

    definition(pair, ops.inputs[0], &[ops.get("slice")?])?;
    definition(
        pair,
        ops.inputs[1],
        &[ops.get("message")?, ops.get("outer-environment")?],
    )?;
    for name in [
        "slice",
        "message",
        "decoded-read",
        "converted-read",
        "outer-environment",
        "map-environment",
    ] {
        // Ordinary reads use reaching-definition edges, never value operands.
        let node = ops.get(name)?;
        if pair
            .pdg
            .edges
            .iter()
            .any(|edge| edge.target == node.id && edge.subkind == "value_operand")
        {
            return Err("Widening saved binding read invents a value operand".into());
        }
    }
    Ok(())
}

fn definition(pair: PairView<'_>, binding: &PdgNode, reads: &[&PdgNode]) -> Result<(), String> {
    for read in reads {
        crate::data_documents::copy_tagged_values::origins::verify_definition(pair, binding, read)?;
    }
    crate::data_documents::value_consumers::verify(pair, binding, reads, "reaching_definition")
}

fn conversion(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    checked: &contract::Checked<'_>,
    ops: &Operations<'_>,
) -> Result<(), String> {
    let summary = crate::lossless_integer_calls::verify_scoped_u32_conversion(
        checked.roles.conversion,
        &[checked.roles.continuation],
        facts,
    )?;
    crate::data_documents::summary_records::Record {
        source: checked.roles.conversion,
        node: ops.get("conversion")?,
        summary,
    }
    .verify_pure(pair, facts, "data.integer.lossless-try-u32-usize")
}
