//! Complete original immediate callback syntax, actuals and selected owned callee.
use super::*;
use crate::concrete_types::{InterfaceObservations, Observation};

struct Parts<'a> {
    receiver: &'a HirNode,
    closure: &'a HirNode,
    binding: &'a HirNode,
    constructor: &'a HirNode,
    error_read: &'a HirNode,
    message_read: &'a HirNode,
    message: &'a crate::ParamFact,
}

pub(super) fn check<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Checked<'a>, String> {
    let parts = parts(node, facts)?;

    let (transfer, mapped_error) = constructor_transfer(parts.constructor, facts)?;

    let input = input(parts.receiver, facts)?;

    let TypeTree::Result { ok, err } = &input else {
        return Err("ID error map requires a complete decoder Result".into());
    };

    if **err != transfer.input || !crate::id_reader_types::id_success(ok, facts)? {
        return Err("ID error map changes generated input error or checked Copy ID".into());
    }

    let output = TypeTree::Result {
        ok: ok.clone(),
        err: Box::new(mapped_error),
    };

    let checked = Checked {
        source: node,
        receiver: parts.receiver,
        closure: parts.closure,
        binding: parts.binding,
        constructor: parts.constructor,
        error_read: parts.error_read,
        message_read: parts.message_read,
        message: parts.message,
        input,
        output,
        storage: TypeTree::Tuple(Vec::new()),
        transfer,
    };

    types(&checked)?;

    Ok(checked)
}

fn parts<'a>(node: &'a HirNode, facts: &'a dyn EvidenceFacts) -> Result<Parts<'a>, String> {
    let [receiver, closure] = node.children.as_slice() else {
        return Err("ID error map requires receiver and immediate closure".into());
    };

    let [binding, constructor] = closure.children.as_slice() else {
        return Err("ID error map closure requires its complete parameter and sole call".into());
    };

    let [error_read, message_read] = constructor.children.as_slice() else {
        return Err("ID error constructor requires whole error then static message".into());
    };

    if !candidate(node, facts)
        || node.expansion.is_some()
        || closure.kind != "expr_closure"
        || closure.expansion.is_some()
        || constructor.kind != "expr_call"
        || constructor.expansion.is_some()
    {
        return Err("ID error map requires original immediate call syntax".into());
    }

    crate::source_evidence::verify_binding_read(
        error_read,
        binding,
        "ID error callback must consume its exact owned parameter",
    )?;

    let root = crate::source_evidence::resolved_local_leaf(message_read)
        .ok_or("ID error callback requires a static-message formal read")?;

    let message = facts
        .signature
        .params
        .iter()
        .find(|parameter| parameter.pat.hir == *root)
        .ok_or("ID error callback static capture is not an original formal")?;

    Ok(Parts {
        receiver,
        closure,
        binding,
        constructor,
        error_read,
        message_read,
        message,
    })
}

fn types(checked: &Checked<'_>) -> Result<(), String> {
    let message = capture::static_message();

    for (source, tree) in [
        (checked.source, &checked.output),
        (checked.binding, &checked.transfer.input),
        (checked.constructor, output_error(&checked.output)?),
        (checked.message_read, &message),
    ] {
        if crate::data_aggregates::token(source)? != crate::data_types::token(tree)? {
            return Err(
                "ID error map source type differs from its selected complete interface".into(),
            );
        }
    }

    Ok(())
}

fn input(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<TypeTree, String> {
    let view = selected_dependency(node, facts)?;
    let ty = selected_result(&view)?;

    if crate::data_aggregates::token(node)? != crate::data_types::token(&ty)? {
        return Err("ID decoder call changes its selected result type".into());
    }

    Ok(ty)
}

fn constructor_transfer(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(crate::named_error_payloads::owned::Transfer, TypeTree), String> {
    let view = selected_dependency(node, facts)?;

    if view.summary().identity != crate::helper_calls::named_errors::OWNED_SUMMARY {
        return Err("ID callback requires the independently proved whole-owned constructor".into());
    }

    let callee = view.frontend();

    let transfer = crate::helper_calls::named_errors::owned_transfer(&callee)?
        .ok_or("ID callback constructor lacks its independently checked transfer")?;

    let ty = selected_result(&view)?;

    Ok((transfer, ty))
}

fn selected_dependency<'a>(
    node: &HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<crate::unit_store::HelperView<'a>, String> {
    let dependency = crate::helper_calls::verify_use(node, facts)?;
    let crate::helper_calls::VerifiedDependency::Shared(view) = dependency else {
        return Err("ID error map helper requires its complete selected owner".into());
    };

    Ok(view)
}

fn selected_result(view: &crate::unit_store::HelperView<'_>) -> Result<TypeTree, String> {
    let callee = view.frontend();
    let interface = callee
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;

    let Observation::Captured { ty } = &interface.result else {
        return Err("ID error map helper lacks complete result observations".into());
    };

    Ok(ty.clone())
}

fn output_error(tree: &TypeTree) -> Result<&TypeTree, String> {
    let TypeTree::Result { err, .. } = tree else {
        return Err("ID error map lacks complete output Result".into());
    };

    Ok(err)
}

pub(super) fn verify(checked: &Checked<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(checked.source, facts)?;

    let syn::Expr::MethodCall(call) = syntax else {
        return Err("ID error map loses its original method call".into());
    };

    if call.method != "map_err"
        || call.args.len() != 1
        || call.turbofish.is_some()
        || !call.attrs.is_empty()
    {
        return Err("ID error map requires one unmodified immediate callback".into());
    }
    corresponding(&*call.receiver, checked.receiver, checked.source, facts)?;

    let argument = call.args.first().expect("single callback");
    corresponding(argument, checked.closure, checked.source, facts)?;
    closure(checked, facts)?;
    static_formal(checked, facts)
}

fn closure(checked: &Checked<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(checked.closure, facts)?;

    let syn::Expr::Closure(closure) = syntax else {
        return Err("ID callback requires original closure syntax".into());
    };

    let (binding, call) = closure_shape(&closure)?;
    corresponding(binding, checked.binding, checked.closure, facts)?;
    corresponding(&*closure.body, checked.constructor, checked.closure, facts)?;

    for (syntax, actual) in call
        .args
        .iter()
        .zip([checked.error_read, checked.message_read])
    {
        corresponding(syntax, actual, checked.closure, facts)?;
    }

    Ok(())
}

fn closure_shape(closure: &syn::ExprClosure) -> Result<(&syn::PatIdent, &syn::ExprCall), String> {
    if !closure.attrs.is_empty()
        || closure.lifetimes.is_some()
        || closure.constness.is_some()
        || closure.movability.is_some()
        || closure.asyncness.is_some()
        || closure.capture.is_some()
        || !matches!(closure.output, syn::ReturnType::Default)
        || closure.inputs.len() != 1
    {
        return Err("ID callback modifiers and callable annotations remain unsupported".into());
    }

    let syn::Pat::Ident(binding) = closure.inputs.first().expect("one callback binding") else {
        return Err("ID callback requires a plain owned binding".into());
    };

    if binding.by_ref.is_some()
        || binding.mutability.is_some()
        || binding.subpat.is_some()
        || !binding.attrs.is_empty()
    {
        return Err("ID callback binding must be immutable and by value".into());
    }

    let syn::Expr::Call(call) = &*closure.body else {
        return Err("ID callback body requires only its selected constructor call".into());
    };

    constructor_shape(call)?;

    Ok((binding, call))
}

fn constructor_shape(call: &syn::ExprCall) -> Result<(), String> {
    if call.args.len() != 2
        || !call.attrs.is_empty()
        || !matches!(&*call.func, syn::Expr::Path(path) if path.qself.is_none()
            && path.attrs.is_empty() && crate::source_evidence::plain_relative_path(&path.path).is_some())
    {
        return Err(
            "ID callback requires an unmodified selected constructor and two actuals".into(),
        );
    }

    if call.args.iter().any(|syntax| {
        !matches!(syntax, syn::Expr::Path(path)
        if path.attrs.is_empty() && path.qself.is_none() && path.path.get_ident().is_some())
    }) {
        return Err("ID callback actuals must be original binding reads".into());
    }

    Ok(())
}

fn static_formal(checked: &Checked<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let syntax: syn::ItemFn =
        syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;

    let argument = syntax
        .sig
        .inputs
        .iter()
        .nth(checked.message.position as usize)
        .ok_or("ID callback static message loses original formal position")?;

    if !matches!(argument, syn::FnArg::Typed(argument)
        if matches!(&*argument.ty, syn::Type::Reference(reference)
            if reference.mutability.is_none()
            && reference.lifetime.as_ref().is_some_and(|lifetime| lifetime.ident == "static")
            && matches!(&*reference.elem, syn::Type::Path(path) if path.qself.is_none()
                && path.path.is_ident("str")))
        && matches!(&*argument.pat, syn::Pat::Ident(binding) if binding.by_ref.is_none()
            && binding.mutability.is_none() && binding.subpat.is_none()))
    {
        return Err("ID callback requires an immutable explicitly static string formal".into());
    }

    crate::source_evidence::verify_binding_read(
        checked.message_read,
        &checked.message.pat,
        "ID callback static message differs from its exact original formal",
    )
}

use crate::source_evidence::verify_child_source as corresponding;

#[cfg(test)]
mod tests;
