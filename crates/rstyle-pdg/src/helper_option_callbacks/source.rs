//! Original nonescaping closure and exact receiver/payload method syntax.
mod bindings;
use crate::option_callbacks::source::corresponding;
use crate::{HirNode, unit_store::EvidenceFacts};

pub(super) fn verify(
    node: &HirNode,
    pattern: &HirNode,
    body: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;
    let syn::Expr::Closure(closure) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Helper callback loses original closure syntax".into());
    };
    if !closure.attrs.is_empty()
        || closure.lifetimes.is_some()
        || closure.constness.is_some()
        || closure.movability.is_some()
        || closure.asyncness.is_some()
        || closure.capture.is_some()
        || !matches!(closure.output, syn::ReturnType::Default)
        || closure.inputs.len() != 1
    {
        return Err("Helper callback requires one immediate unmodified closure parameter".into());
    }

    let parameter = parameter(
        closure.inputs.first().expect("one parameter"),
        pattern,
        node,
        facts,
    )?;
    corresponding(&*closure.body, body, node, facts)?;
    let (receiver, value) = method(&closure.body, body, node, facts)?;
    bindings::receiver(&receiver, node, facts)?;
    bindings::payload(&value, &parameter)
}

fn parameter(
    syntax: &syn::Pat,
    pattern: &HirNode,
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<syn::Ident, String> {
    corresponding(syntax, pattern, node, facts)?;
    let (syntax, binding) = if pattern.kind == "pat_shared_copy" {
        let syn::Pat::Reference(reference) = syntax else {
            return Err("Borrowed helper callback requires original shared pattern".into());
        };
        if reference.mutability.is_some() || !reference.attrs.is_empty() {
            return Err("Callback shared pattern must be immutable".into());
        }
        (&*reference.pat, &pattern.children[0])
    } else {
        (syntax, pattern)
    };

    let syn::Pat::Ident(ident) = syntax else {
        return Err("Helper callback requires a plain payload binding".into());
    };
    if ident.by_ref.is_some()
        || ident.mutability.is_some()
        || ident.subpat.is_some()
        || !ident.attrs.is_empty()
    {
        return Err("Helper callback payload binding must be unmodified".into());
    }

    corresponding(ident, binding, node, facts)?;
    Ok(ident.ident.clone())
}

fn method(
    syntax: &syn::Expr,
    body: &HirNode,
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(syn::Ident, syn::Ident), String> {
    let syn::Expr::MethodCall(method) = crate::data_aggregates::ungroup(syntax) else {
        return Err("Helper callback requires an original method body".into());
    };
    if !method.attrs.is_empty() || method.turbofish.is_some() || method.args.len() != 1 {
        return Err("Helper callback method requires one unsubstituted argument".into());
    }

    let callee =
        crate::helper_calls::direct_resolution(body).ok_or("Missing resolved callback helper")?;
    let path = callee["path"]
        .as_str()
        .ok_or("Missing callback helper path")?;
    if path.rsplit("::").next() != Some(method.method.to_string().as_str()) {
        return Err("Helper callback original method disagrees with its resolution".into());
    }

    let [receiver, value] = body.children.as_slice() else {
        return Err("Helper callback loses its receiver/payload source sites".into());
    };
    let receiver = binding_path(&method.receiver, receiver, node, facts)?;
    let value = binding_path(&method.args[0], value, node, facts)?;
    Ok((receiver, value))
}

fn binding_path(
    syntax: &syn::Expr,
    child: &HirNode,
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<syn::Ident, String> {
    let syn::Expr::Path(path) = crate::data_aggregates::ungroup(syntax) else {
        return Err("Helper callback body permits only exact binding reads".into());
    };
    if !path.attrs.is_empty() || path.qself.is_some() {
        return Err("Helper callback body requires unqualified binding paths".into());
    }

    let ident = path
        .path
        .get_ident()
        .ok_or("Callback path is not one binding")?;
    corresponding(syntax, child, node, facts)?;
    Ok(ident.clone())
}
