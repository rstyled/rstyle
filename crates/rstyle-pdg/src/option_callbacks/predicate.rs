//! Bounded scalar equality bodies and complete shared scalar environments.

use super::*;
use crate::{HirNode, concrete_types::TypeTree, unit_store::EvidenceFacts};

pub struct Predicate<'a> {
    pub binding: &'a HirNode,
    pub body: &'a HirNode,
    pub capture: &'a crate::BindingFact,
    pub captured_value: &'a HirNode,
}

/// Checks the predicate itself, without authorizing a callable value, a library
/// invocation or a graph. Invocation order and Some guards require other proof.
pub fn verify_predicate<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Predicate<'a>, String> {
    let [binding, body] = node.children.as_slice() else {
        return Err("Callback requires its complete parameter and body".into());
    };
    if node.kind != "expr_closure" || node.expansion.is_some() || node.is_unsafe_block {
        return Err("Callback requires an original synchronous closure".into());
    }

    let parameter = if binding.kind == "pat_shared_copy"
        && crate::extraction_profile::facts_support(facts, "rust.byte-iterator-position")
    {
        let [inner] = binding.children.as_slice() else {
            return Err("Shared callback pattern requires one complete binding".into());
        };
        if crate::data_aggregates::token(binding)?
            != crate::data_types::token(&TypeTree::Shared(Box::new(TypeTree::Scalar("u8".into()))))?
        {
            return Err("Shared callback pattern requires a shared byte".into());
        }
        inner
    } else {
        binding
    };
    parameter_check(node, parameter, facts)?;
    let captured = equality(parameter, body)?;
    let capture = facts
        .bindings
        .iter()
        .find(|record| {
            Some(&record.hir) == captured.res.as_ref().and_then(|res| res.target.as_ref())
        })
        .ok_or("Callback capture has no enclosing binding")?;
    if capture.hir == parameter.hir || capture.ty.normalized.as_deref() != Some("u8") {
        return Err("Callback requires a distinct primitive byte capture".into());
    }
    environment(node, capture)?;
    source::verify(node, binding, body, facts)?;

    Ok(Predicate {
        binding: parameter,
        body,
        capture,
        captured_value: captured,
    })
}

fn parameter_check(
    closure: &HirNode,
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let record = facts.bindings.iter().find(|record| record.hir == node.hir);
    let scope = format!(
        "scope:source:callback:{}:{}",
        closure.span.start, closure.span.end
    );
    if node.kind != "pat_binding"
        || node.binding.is_none()
        || node.binding_mut != Some(false)
        || node.param_index.is_some()
        || !node.children.is_empty()
        || node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some("u8")
        || !record.is_some_and(|record| {
            !record.is_param
                && !record.is_mut
                && Some(&record.symbol) == node.binding.as_ref()
                && record.ty.normalized.as_deref() == Some("u8")
                && record.scope == scope
                && record.decl_span == node.span
        })
    {
        return Err("Callback requires one immutable by-value byte parameter".into());
    }
    Ok(())
}

fn equality<'a>(binding: &HirNode, body: &'a HirNode) -> Result<&'a HirNode, String> {
    let [left, right] = body.children.as_slice() else {
        return Err("Callback requires a complete scalar equality".into());
    };
    if body.kind != "expr_binary"
        || body.expansion.is_some()
        || !body.dispatch.as_ref().is_some_and(|dispatch| {
            dispatch.kind == "builtin"
                && dispatch.op == "=="
                && dispatch.operand_tys == ["u8", "u8"]
                && dispatch.result_ty == "bool"
        })
        || body.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some("bool")
    {
        return Err("Callback requires built-in byte equality returning bool".into());
    }
    path(left)?;
    path(right)?;

    match (
        left.res.as_ref().and_then(|res| res.target.as_ref()) == Some(&binding.hir),
        right.res.as_ref().and_then(|res| res.target.as_ref()) == Some(&binding.hir),
    ) {
        (true, false) => Ok(right),
        (false, true) => Ok(left),
        _ => Err("Callback must compare its payload with exactly one capture".into()),
    }
}

fn path(node: &HirNode) -> Result<(), String> {
    if node.kind != "expr_path"
        || node.expansion.is_some()
        || !node.children.is_empty()
        || node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some("u8")
        || !node
            .res
            .as_ref()
            .is_some_and(|res| res.kind == "local" && res.target.is_some())
    {
        return Err("Callback operands must be original resolved byte bindings".into());
    }
    Ok(())
}

fn environment(node: &HirNode, capture: &crate::BindingFact) -> Result<(), String> {
    let resolution = node.res.as_ref().ok_or("Missing callback environment")?;
    if resolution.kind != ENVIRONMENT_KIND || resolution.target.is_some() {
        return Err("Callback lacks compiler environment observations".into());
    }
    let environment: Environment =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;

    let expected = Capture {
        root: capture.hir.clone(),
        mode: CaptureMode::Shared,
        projections: 0,
    };
    let storage = Observation::Captured {
        ty: TypeTree::Tuple(vec![TypeTree::Shared(Box::new(TypeTree::Scalar(
            "u8".into(),
        )))]),
    };
    if environment.captures != [expected]
        || environment.stored_type != storage
        || environment.shared_formal.is_some()
    {
        return Err(
            "Callback requires exactly one shared byte capture with drop-free storage".into(),
        );
    }
    Ok(())
}
