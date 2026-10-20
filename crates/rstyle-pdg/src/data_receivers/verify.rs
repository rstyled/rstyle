//! Independent receiver inventories and concrete shared-reference transitions.

use super::*;
use crate::{
    HirFnFacts, HirNode,
    concrete_types::{InterfaceObservations, Observation, TypeTree},
    extraction_profile::ExtractionProfile,
};
use std::collections::BTreeSet;

pub fn verify(facts: &HirFnFacts) -> Result<Capture, String> {
    check(facts, true)
}

/// Validate observations without claiming original-source correspondence.
/// Expanded functions remain subject to the full verifier before admission.
pub fn verify_capture(facts: &HirFnFacts) -> Result<Capture, String> {
    check(facts, false)
}

fn check(facts: &HirFnFacts, original_source: bool) -> Result<Capture, String> {
    let value = facts.build.frontend_options.get(KEY);
    if !ExtractionProfile::from_facts(facts)?.is_data() {
        return if value.is_none() {
            Ok(empty())
        } else {
            Err("Receiver inventory is restricted to the Data profile".into())
        };
    }
    let mut methods = Vec::new();
    crate::frontend::collect_kind(&facts.body, "expr_method_call", &mut methods);
    let capture: Capture = match value {
        Some(value) => serde_json::from_value(value.clone()).map_err(|error| error.to_string())?,
        None if methods.is_empty() => empty(),
        None => return Err("Missing Data method receiver inventory".into()),
    };
    check_inventory(&capture, &methods)?;
    let source = (original_source && methods.iter().any(|node| node.expansion.is_none()))
        .then(|| crate::source_evidence::capture(facts))
        .transpose()?;
    for node in methods {
        let record = capture
            .methods
            .iter()
            .find(|record| record.call == node.hir)
            .ok_or("Missing exact method receiver record")?;
        check_receiver_identity(record, node)?;
        if original_source {
            check_correspondence(node, source.as_ref())
                .map_err(|error| format!("{} at {:?}: {error}", facts.name, node.span))?;
        }
        check_types(record, node)
            .map_err(|error| format!("{} at {:?}: {error}", facts.name, node.span))?;
        check_adjustments(record)?;
    }

    Ok(capture)
}

fn empty() -> Capture {
    Capture {
        identity: IDENTITY.into(),
        methods: Vec::new(),
    }
}

fn check_inventory(capture: &Capture, methods: &[&HirNode]) -> Result<(), String> {
    if capture.identity != IDENTITY
        || capture.methods.len() != methods.len()
        || capture.methods.len() > MAX_RECORDS
    {
        return Err("Method receiver identity, inventory or budget disagrees".into());
    }
    let ids: BTreeSet<_> = capture
        .methods
        .iter()
        .map(|record| (record.call.krate, record.call.owner, record.call.local))
        .collect();
    if ids.len() != capture.methods.len() {
        return Err("Duplicate method receiver record".into());
    }
    Ok(())
}

fn check_receiver_identity(record: &Method, node: &HirNode) -> Result<(), String> {
    let receiver = node
        .children
        .first()
        .ok_or("Missing method receiver operand")?;
    if record.source_span != node.span || record.receiver != receiver.hir {
        return Err("Method receiver source/HIR identity or span disagrees".into());
    }

    Ok(())
}

fn check_correspondence(
    node: &HirNode,
    source: Option<&crate::source_evidence::Correspondence>,
) -> Result<(), String> {
    if node.expansion.is_some() {
        return Ok(());
    }

    let source = source.ok_or("Missing original-source method inventory")?;
    let macro_argument = source.entries.iter().any(|entry| {
        entry.kind == "source_macro"
            && entry.span.start <= node.span.start
            && node.span.end <= entry.span.end
    });
    if !macro_argument
        && source
            .entries
            .iter()
            .filter(|entry| {
                entry.kind == "expr_method_call" && entry.hir.as_ref() == Some(&node.hir)
            })
            .count()
            != 1
    {
        return Err("No unique original-source method correspondence".into());
    }
    Ok(())
}

fn check_types(record: &Method, node: &HirNode) -> Result<(), String> {
    let receiver = node
        .children
        .first()
        .ok_or("Missing method receiver operand")?;
    check_value_type(&record.receiver_type, receiver)?;
    check_value_type(&record.signature_result, node)?;
    check_registered_signature(record, node)?;

    let observations = std::iter::once(&record.receiver_type)
        .chain([&record.adjusted_type])
        .chain(
            record
                .adjustments
                .iter()
                .map(|adjustment| &adjustment.target),
        )
        .chain(&record.signature_inputs)
        .cloned()
        .collect();
    InterfaceObservations {
        identity: crate::concrete_types::IDENTITY.into(),
        parameters: observations,
        result: record.signature_result.clone(),
    }
    .validate()
}

fn check_registered_signature(record: &Method, node: &HirNode) -> Result<(), String> {
    let Some(summary) = crate::call_summaries::resolve(node) else {
        return Ok(());
    };
    let expected: Vec<_> = summary
        .argument_types
        .iter()
        .map(|token| Observation::Captured {
            ty: TypeTree::Scalar(token.clone()),
        })
        .collect();
    if record.signature_inputs != expected {
        return Err("Observed method signature disagrees with registered scalar summary".into());
    }
    Ok(())
}

fn check_value_type(observation: &Observation, node: &HirNode) -> Result<(), String> {
    let expected = match observation {
        Observation::Captured { ty } => Some(crate::data_types::token(ty)?),
        Observation::Unsupported { .. } => None,
    };
    if node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != expected.as_deref() {
        return Err(format!(
            "Concrete method receiver/result disagrees with HIR value type: observed {observation:?}, HIR {:?}",
            node.ty
        ));
    }
    Ok(())
}

fn check_adjustments(record: &Method) -> Result<(), String> {
    if record.adjustments.len() > MAX_ADJUSTMENTS {
        return Err("Method receiver adjustment budget exceeded".into());
    }
    let mut previous = &record.receiver_type;
    for adjustment in &record.adjustments {
        check_transition(previous, adjustment)?;
        previous = &adjustment.target;
    }

    if previous != &record.adjusted_type {
        return Err("Method receiver adjustment endpoint disagrees".into());
    }
    Ok(())
}

fn check_transition(previous: &Observation, adjustment: &Adjustment) -> Result<(), String> {
    if let Kind::Unsupported(reason) = &adjustment.kind {
        return if matches!(
            reason.as_str(),
            "never_coercion"
                | "overloaded_dereference"
                | "pin_dereference"
                | "raw_borrow"
                | "pin_borrow"
                | "pointer_coercion"
                | "generic_reborrow"
        ) {
            Ok(())
        } else {
            Err("Unregistered unsupported receiver conversion".into())
        };
    }
    if adjustment.kind == Kind::MutableBorrow {
        return if matches!(adjustment.target, Observation::Unsupported { .. }) {
            Ok(())
        } else {
            Err("Mutable receiver target cannot be a supported concrete type".into())
        };
    }

    let (Observation::Captured { ty: before }, Observation::Captured { ty: after }) =
        (previous, &adjustment.target)
    else {
        return Ok(());
    };
    let valid = match &adjustment.kind {
        Kind::BuiltinDereference => matches!(before, TypeTree::Shared(inner) if &**inner == after),
        Kind::SharedBorrow => matches!(after, TypeTree::Shared(inner) if &**inner == before),
        _ => false,
    };
    if !valid {
        return Err("Concrete shared receiver transition disagrees".into());
    }
    Ok(())
}
