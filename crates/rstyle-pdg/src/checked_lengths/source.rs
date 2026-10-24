use super::*;
use syn::spanned::Spanned;

pub(super) fn verify(node: &HirNode, kind: Kind, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;
    let syn::Expr::MethodCall(call) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Checked length arithmetic requires original method syntax".into());
    };
    if call.method != kind.method() || call.args.len() != 1 || call.turbofish.is_some() {
        return Err("Checked length source disagrees with its primitive operation".into());
    }

    let [receiver, argument] = node.children.as_slice() else {
        return Err("Checked length arithmetic requires exact receiver and argument".into());
    };
    let bytes = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing checked length source")?;
    for (original, observed) in [(&*call.receiver, receiver), (&call.args[0], argument)] {
        let span = crate::data_aggregates::ungroup(original).span();
        let expected = crate::ByteSpan {
            start: node.span.start
                + crate::source_evidence::byte_offset(bytes, span.start()) as u32,
            end: node.span.start + crate::source_evidence::byte_offset(bytes, span.end()) as u32,
        };
        if !crate::source_evidence::corresponding_span(&facts.source.bytes, expected, observed.span)
        {
            return Err(
                "Checked length arithmetic loses original ordered operand correspondence".into(),
            );
        }
    }
    Ok(())
}
