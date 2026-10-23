use super::*;
use syn::spanned::Spanned;

pub(super) fn verify(node: &HirNode, kind: Kind, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;
    let source = crate::data_aggregates::ungroup(&syntax);
    let arguments = if kind.implementation().is_some() {
        vec![crate::source_evidence::primitive_conversion(
            source,
            if kind == Kind::Narrow { "u16" } else { "usize" },
        )?]
    } else {
        method(source, kind)?
    };

    let bytes = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing integer call source")?;
    if arguments.len() != node.children.len() {
        return Err("Integer source loses operand count".into());
    }
    for (argument, operand) in arguments.into_iter().zip(&node.children) {
        let span = crate::data_aggregates::ungroup(argument).span();
        let expected = crate::ByteSpan {
            start: node.span.start
                + crate::source_evidence::byte_offset(bytes, span.start()) as u32,
            end: node.span.start + crate::source_evidence::byte_offset(bytes, span.end()) as u32,
        };

        if !crate::source_evidence::corresponding_span(&facts.source.bytes, expected, operand.span)
        {
            return Err("Integer operation loses its original operand".into());
        }
    }
    Ok(())
}

fn method(source: &syn::Expr, kind: Kind) -> Result<Vec<&syn::Expr>, String> {
    let syn::Expr::MethodCall(call) = source else {
        return Err("Integer method requires original method syntax".into());
    };
    let name = if kind == Kind::Count {
        "count_ones"
    } else if kind == Kind::CheckedSub {
        "checked_sub"
    } else {
        "unwrap"
    };
    if call.method != name
        || call.args.len() != usize::from(kind == Kind::CheckedSub)
        || call.turbofish.is_some()
    {
        return Err("Integer method syntax disagrees with reviewed operation".into());
    }
    Ok(std::iter::once(call.receiver.as_ref())
        .chain(call.args.iter())
        .collect())
}
