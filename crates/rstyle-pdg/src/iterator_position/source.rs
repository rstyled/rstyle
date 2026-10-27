use super::*;

pub(super) fn verify(
    node: &HirNode,
    iterator: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    invocation(node, "position", facts)?;
    invocation(iterator, "iter", facts)
}

fn invocation(node: &HirNode, method: &str, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;
    let syn::Expr::MethodCall(call) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Byte iterator requires original method syntax".into());
    };
    if call.method != method
        || call.turbofish.is_some()
        || !call.attrs.is_empty()
        || call.args.len() != usize::from(method == "position")
    {
        return Err("Byte iterator loses original arguments/modifiers".into());
    }

    for (syntax, child) in std::iter::once(&*call.receiver)
        .chain(call.args.iter())
        .zip(&node.children)
    {
        crate::option_callbacks::source::corresponding(syntax, child, node, facts)?;
    }
    Ok(())
}
