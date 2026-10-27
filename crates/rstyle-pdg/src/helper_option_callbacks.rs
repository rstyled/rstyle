//! Immediate byte callbacks with a completely verified shared-receiver helper.
mod source;

use crate::{HirNode, unit_store::EvidenceFacts};
pub(crate) const CAPABILITY: &str = "rust.shared-helper-option-callbacks";
pub(crate) const ENVIRONMENT: &str = "callback.shared-helper-environment";
pub(crate) const SELECTOR: &str = "callback.helper-option-some";
pub(crate) const RESULT: &str = "callback.helper-option-result";

pub(crate) struct Checked<'a> {
    pub arguments: crate::option_callbacks::Arguments<'a>,
    pub binding: &'a HirNode,
    pub body: &'a HirNode,
    pub borrowed: bool,
}

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && node.kind == "expr_method_call"
        && node.children.last().is_some_and(|closure| {
            closure.kind == "expr_closure"
                && closure
                    .children
                    .last()
                    .is_some_and(|body| body.kind == "expr_method_call")
        })
        && crate::data_library::callee(node).is_ok_and(|callee| {
            matches!(
                callee.path.as_str(),
                "std::option::Option::<T>::map_or" | "core::option::Option::<T>::map_or"
            )
        })
}

pub(crate) fn sources(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    method_sources(facts, candidate)
}

pub(crate) fn method_sources(
    facts: &dyn EvidenceFacts,
    candidate: impl Fn(&HirNode, &dyn EvidenceFacts) -> bool,
) -> Vec<&HirNode> {
    let mut nodes = Vec::new();
    crate::frontend::collect_kind(&facts.body, "expr_method_call", &mut nodes);

    nodes
        .into_iter()
        .filter(|node| candidate(node, facts))
        .collect()
}

pub(crate) fn verify_source<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Checked<'a>, String> {
    if !candidate(node, facts) {
        return Err("Shared helper callback requires its capability and immediate syntax".into());
    }
    let arguments = crate::option_callbacks::verify_arguments(node, facts)?;
    let [pattern, body] = arguments.callback.children.as_slice() else {
        return Err("Helper callback loses its complete parameter or body".into());
    };

    let borrowed = !crate::checked_options::owned(arguments.receiver);
    let binding = parameter(arguments.callback, pattern, borrowed, facts)?;

    helper_body(arguments.callback, binding, body, facts)?;
    source::verify(arguments.callback, pattern, body, facts)?;

    Ok(Checked {
        arguments,
        binding,
        body,
        borrowed,
    })
}

fn helper_body(
    closure: &HirNode,
    binding: &HirNode,
    body: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let [receiver, value] = body.children.as_slice() else {
        return Err("Helper callback requires exactly its receiver and byte argument".into());
    };
    if body.expansion.is_some()
        || body.is_unsafe_block
        || crate::data_aggregates::token(body)? != "bool"
        || !body
            .res
            .as_ref()
            .is_some_and(|res| res.kind == "resolved_inherent_callee")
        || receiver.kind != "expr_path"
        || !receiver.children.is_empty()
        || receiver.expansion.is_some()
        || value.kind != "expr_path"
        || !value.children.is_empty()
        || value.expansion.is_some()
        || crate::data_aggregates::token(value)? != "u8"
        || value.res.as_ref().and_then(|res| res.target.as_ref()) != Some(&binding.hir)
    {
        return Err(
            "Helper callback requires a resolved bool method on its exact byte binding".into(),
        );
    }

    let captured = crate::option_callbacks::captures::verify_structure(closure, facts)?;
    if receiver.res.as_ref().and_then(|res| res.target.as_ref()) != Some(&captured.root)
        || crate::data_aggregates::token(receiver)? != captured.token
    {
        return Err("Helper callback receiver differs from its captured immutable formal".into());
    }

    Ok(())
}

fn parameter<'a>(
    closure: &HirNode,
    pattern: &'a HirNode,
    borrowed: bool,
    facts: &dyn EvidenceFacts,
) -> Result<&'a HirNode, String> {
    if closure.kind != "expr_closure" || closure.expansion.is_some() || closure.is_unsafe_block {
        return Err("Helper callback requires an original synchronous closure".into());
    }
    let binding = payload_binding(pattern, borrowed)?;
    let scope = format!(
        "scope:source:callback:{}:{}",
        closure.span.start, closure.span.end
    );

    if binding.kind != "pat_binding"
        || binding.binding.is_none()
        || binding.binding_mut != Some(false)
        || binding.param_index.is_some()
        || !binding.children.is_empty()
        || crate::data_aggregates::token(binding)? != "u8"
        || !facts.bindings.iter().any(|record| {
            record.hir == binding.hir
                && !record.is_param
                && !record.is_mut
                && Some(&record.symbol) == binding.binding.as_ref()
                && record.ty.normalized.as_deref() == Some("u8")
                && record.scope == scope
                && record.decl_span == binding.span
        })
    {
        return Err("Helper callback requires one immutable original byte binding".into());
    }

    Ok(binding)
}

fn payload_binding(pattern: &HirNode, borrowed: bool) -> Result<&HirNode, String> {
    let binding = if borrowed {
        let [binding] = pattern.children.as_slice() else {
            return Err("Borrowed helper callback requires exactly one copied byte binding".into());
        };
        let expected = crate::data_types::token(&crate::concrete_types::TypeTree::Shared(
            Box::new(crate::concrete_types::TypeTree::Scalar("u8".into())),
        ))?;
        if pattern.kind != "pat_shared_copy" || crate::data_aggregates::token(pattern)? != expected
        {
            return Err("Borrowed helper callback requires a shared byte pattern".into());
        }
        binding
    } else {
        pattern
    };
    Ok(binding)
}

pub(crate) fn verify<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Checked<'a>, String> {
    let checked = verify_source(node, facts)?;
    crate::option_callbacks::captures::verify_shared_formal(
        checked.arguments.callback,
        &checked.body.children[0],
        facts,
    )?;

    let dependency = crate::helper_calls::verify_use(checked.body, facts)?;
    let summary = dependency.summary();
    let receiver = crate::data_aggregates::token(&checked.body.children[0])?;

    if summary.argument_types != [receiver, "u8"]
        || summary.result_type != "bool"
        || summary.mutates_arguments
        || summary.memory_effects != summary.memory_reads.is_some()
        || summary.may_panic
        || !summary.always_returns
        || summary.reference_outputs.is_some()
        || summary.component_outputs.is_some()
    {
        return Err(
            "Immediate helper callback requires a complete returning read-only bool helper".into(),
        );
    }

    Ok(checked)
}

pub(crate) fn owner<'a>(
    binding: &HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Checked<'a>, String> {
    for call in sources(facts) {
        let checked = verify_source(call, facts)?;
        if checked.binding.hir == binding.hir {
            return Ok(checked);
        }
    }
    Err("Payload lacks its immediate shared-helper callback owner".into())
}

pub(crate) fn borrowed_sources(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    sources(facts)
        .into_iter()
        .filter_map(|call| verify_source(call, facts).ok())
        .filter(|checked| checked.borrowed)
        .map(|checked| checked.binding)
        .collect()
}

pub(crate) fn verify_environment_event(
    closure: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let call = sources(facts)
        .into_iter()
        .find(|call| {
            call.children
                .last()
                .is_some_and(|node| node.hir == closure.hir)
        })
        .ok_or("Shared capture environment lacks its immediate helper callback owner")?;
    verify(call, facts).map(|_| ())
}

pub(crate) fn locations(
    binding: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Vec<crate::field_locations::derived::Located>, String> {
    let checked = owner(binding, facts)?;
    if !checked.borrowed {
        return Err("Owned callback payload has no borrowed location".into());
    }

    let roots = crate::data_origins::origins(checked.arguments.receiver, facts)?;
    if roots.is_empty() {
        return Err("Helper callback payload lacks conditional origins".into());
    }

    let scalar = crate::data_types::parse("u8")?;
    crate::field_locations::derived::from_origins(scalar, facts, roots)
}
