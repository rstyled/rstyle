//! Receiver observations for an already checked constructor-transport source.
//! This route grants neither Result method semantics nor helper permission.
use super::*;
use crate::{
    HirNode,
    concrete_types::{Observation, TypeTree},
    result_maps::source::CheckedTransportSource,
    unit_store::EvidenceFacts,
};

pub(super) fn verify(
    facts: &dyn EvidenceFacts,
    capture: &Capture,
    methods: &[&HirNode],
) -> Result<bool, String> {
    if !methods
        .iter()
        .any(|node| crate::result_maps::candidate(node, facts))
    {
        return Ok(false);
    }
    let resolution = facts.nominal_resolution()?;
    let Some(source) = resolution.transport_source() else {
        return Ok(false);
    };
    source.verify(facts.facts())?;
    inventory(source, capture, methods)?;

    Ok(true)
}

fn inventory(
    source: &CheckedTransportSource,
    capture: &Capture,
    methods: &[&HirNode],
) -> Result<(), String> {
    let request = source.request();
    let [input, intermediate, output] = request.result_types();
    if methods.len() != 2 || capture.methods.len() != 2 || capture.identity != IDENTITY {
        return Err(
            "Constructor transport receiver inventory must contain exactly both methods".into(),
        );
    }
    for ((hir, types), constructor) in request
        .map_hirs()
        .into_iter()
        .zip([[input, intermediate], [intermediate, output]])
        .zip(request.constructors())
    {
        let nodes: Vec<_> = methods
            .iter()
            .filter(|node| node.hir == *hir)
            .copied()
            .collect();
        let [node] = nodes.as_slice() else {
            return Err("Transport receiver lacks its unique exact source method".into());
        };
        let records: Vec<_> = capture
            .methods
            .iter()
            .filter(|record| record.call == *hir)
            .collect();
        let [record] = records.as_slice() else {
            return Err("Transport receiver has missing or duplicate raw observations".into());
        };
        if !constructor.local || constructor.definition_identity.is_empty() {
            return Err(
                "Transport receiver lacks its exact checked constructor substitution".into(),
            );
        }
        correspondence(source, node)?;
        super::verify::check_receiver_identity(record, node)?;
        record_types(record, types)?;
    }

    Ok(())
}

fn correspondence(source: &CheckedTransportSource, node: &HirNode) -> Result<(), String> {
    let [receiver, callback] = node.children.as_slice() else {
        return Err("Constructor transport method requires receiver and constructor".into());
    };
    for (node, kind) in [
        (node, "expr_method_call"),
        (receiver, receiver.kind.as_str()),
        (callback, "expr_path"),
    ] {
        if node.kind != kind
            || source
                .member()
                .roles()
                .iter()
                .filter(|role| role.hir() == &node.hir && role.kind() == kind)
                .count()
                != 1
        {
            return Err(
                "Transport receiver lost exact original template-role correspondence".into(),
            );
        }
    }
    Ok(())
}

fn record_types(record: &Method, types: [&TypeTree; 2]) -> Result<(), String> {
    let [input, output] = types;
    let captured = |observation: &Observation, expected: &TypeTree| matches!(observation, Observation::Captured { ty } if ty == expected);
    if !record.adjustments.is_empty()
        || !captured(&record.receiver_type, input)
        || !captured(&record.adjusted_type, input)
        || !captured(&record.signature_result, output)
        || !matches!(record.signature_inputs.as_slice(), [receiver, Observation::Unsupported { .. }]
            if captured(receiver, input))
    {
        return Err(
            "Constructor transport receiver/signature types or adjustments disagree".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests;
