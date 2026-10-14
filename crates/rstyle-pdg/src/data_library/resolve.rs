//! Narrow shared-metadata operations; no source spelling or purity inference.

use super::*;
use crate::{
    call_summaries::Summary,
    concrete_types::{Observation, TypeTree},
    data_receivers::Kind,
};

pub fn resolve(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Option<Summary>, String> {
    let Some(op) = operation(node) else {
        return Ok(None);
    };
    let Some(token) = source_token(node, facts, op)? else {
        return Ok(None);
    };

    crate::data_frontend::verify(facts)?;
    let receiver = node.children.first().ok_or("Missing library receiver")?;
    if crate::data_origins::origins(receiver, facts)?.is_empty() {
        return Ok(None);
    }
    Ok(Some(metadata_summary(op, token)))
}

/// Source correspondence precedes helper linking; this never authorizes admission.
pub(crate) fn source_resolved(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<bool, String> {
    let Some(op) = operation(node) else {
        return Ok(false);
    };
    Ok(source_token(node, facts, op)?.is_some())
}

fn source_token(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    op: &Operation,
) -> Result<Option<String>, String> {
    if !crate::extraction_profile::ExtractionProfile::from_facts(facts)?.is_data() {
        return Ok(None);
    }
    let artifacts = super::verify_evidence(facts)?;
    if !super::verify::registered_build_with(facts, &artifacts)? {
        return Ok(None);
    }
    let capture = crate::data_receivers::verify(facts)?;
    let record = capture
        .methods
        .iter()
        .find(|method| method.call == node.hir)
        .ok_or("Missing verified library receiver")?;
    if !supported_receiver(record, op) || !original_invocation(node, facts, op)? {
        return Ok(None);
    }
    let Observation::Captured { ty } = &record.receiver_type else {
        return Ok(None);
    };

    let token = crate::data_types::token(ty)?;
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing registered method dispatch")?;
    if dispatch.operand_tys != [token.clone()] || dispatch.result_ty != op.result {
        return Err(
            "Registered length dispatch types disagree with concrete receiver/result".into(),
        );
    }

    Ok(Some(token))
}

fn metadata_summary(op: &Operation, token: String) -> Summary {
    Summary {
        id: op.id.clone(),
        version: "1".into(),
        callee_path: op.path.clone(),
        callee_crate: "core".into(),
        argument_types: vec![token],
        result_type: op.result.clone(),
        memory_effects: false,
        mutates_arguments: false,
        may_panic: false,
        always_returns: true,
        overflow_behavior: "not_applicable".into(),
    }
}

fn supported_receiver(record: &crate::data_receivers::Method, op: &Operation) -> bool {
    let Observation::Captured {
        ty: TypeTree::Shared(referent),
    } = &record.receiver_type
    else {
        return false;
    };
    let type_ok = match (&**referent, op.referent.as_str()) {
        (TypeTree::Str, "str") => true,
        (TypeTree::Slice(element), "slice") => {
            !crate::data_origins::shared(&crate::data_types::TypeToken::from_tree(element))
        }
        _ => false,
    };

    let kinds: Vec<_> = record.adjustments.iter().map(|step| &step.kind).collect();
    let adjustments_ok =
        kinds.is_empty() || kinds == [&Kind::BuiltinDereference, &Kind::SharedBorrow];
    type_ok && adjustments_ok && record.adjusted_type == record.receiver_type
}

fn original_invocation(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    op: &Operation,
) -> Result<bool, String> {
    if node.expansion.is_some() || node.children.len() != 1 {
        return Ok(false);
    }
    let bytes = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original method source")?;
    let expression: syn::Expr = syn::parse_str(bytes).map_err(|error| error.to_string())?;
    let expression = unwrap_parentheses(&expression);

    let syn::Expr::MethodCall(call) = expression else {
        return Ok(false);
    };
    Ok(call.args.is_empty()
        && call.turbofish.is_none()
        && op.path.ends_with(&format!("::{}", call.method)))
}

fn unwrap_parentheses(mut expression: &syn::Expr) -> &syn::Expr {
    while let syn::Expr::Paren(inner) = expression {
        expression = &inner.expr;
    }
    expression
}
