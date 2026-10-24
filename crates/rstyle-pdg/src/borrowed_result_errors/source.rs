//! Independent original syntax and exact receiver/argument correspondence.
use super::*;

pub(super) fn method(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;
    let call = method_expression(&syntax)?;
    crate::source_evidence::verify_child_source(&*call.receiver, &node.children[0], node, facts)?;
    crate::source_evidence::verify_child_source(&call.args[0], &node.children[1], node, facts)
}

pub(super) fn method_expression(syntax: &syn::Expr) -> Result<&syn::ExprMethodCall, String> {
    let syn::Expr::MethodCall(call) = crate::data_aggregates::ungroup(syntax) else {
        return Err("Eager ok_or requires original method syntax".into());
    };
    if call.method != "ok_or"
        || call.args.len() != 1
        || call.turbofish.is_some()
        || !call.attrs.is_empty()
    {
        return Err("Eager ok_or requires one original fresh error argument".into());
    }

    Ok(call)
}

pub(super) fn variant(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(syn::TypePath, String), String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;
    qualified_variant(syntax)
}

pub(super) fn qualified_variant(syntax: syn::Expr) -> Result<(syn::TypePath, String), String> {
    let syn::Expr::Path(path) = syntax else {
        return Err("Fresh error requires a direct original variant path".into());
    };
    let (owner, name) = crate::source_evidence::qualified_variant(path)
        .map_err(|_| "Fresh error requires an enum-qualified nongeneric source path".to_owned())?;
    Ok((
        syn::TypePath {
            qself: None,
            path: owner,
        },
        name,
    ))
}
