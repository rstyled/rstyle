use super::*;
use syn::spanned::Spanned;

pub(super) fn verify(node: &HirNode, kind: Kind, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let original = crate::source_evidence::original_expression(node, facts)?;
    let arguments = operands(crate::data_aggregates::ungroup(&original), kind)?;
    let bytes = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original lossless call source")?;
    if arguments.len() != node.children.len() {
        return Err("Lossless call source drops an evaluated operand".into());
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
            return Err("Lossless call loses original evaluation order/operand".into());
        }
    }
    Ok(())
}

pub(super) fn operands(source: &syn::Expr, kind: Kind) -> Result<Vec<&syn::Expr>, String> {
    if kind.is_conversion() {
        Ok(vec![crate::source_evidence::primitive_conversion(
            source,
            kind.output(),
        )?])
    } else {
        expect(source)
    }
}

fn expect(source: &syn::Expr) -> Result<Vec<&syn::Expr>, String> {
    let syn::Expr::MethodCall(call) = source else {
        return Err("Lossless expect requires original method syntax".into());
    };
    if call.method != "expect" || call.args.len() != 1 || call.turbofish.is_some() {
        return Err("Lossless expect source differs from reviewed method".into());
    }

    let message = crate::data_aggregates::ungroup(&call.args[0]);
    if !matches!(message, syn::Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Str(_))) {
        return Err("Lossless expect requires an original literal message operand".into());
    }
    // Return both operands. The caller must retain ordinary evaluation and edges
    // for the message even though the proved producer cannot enter expect's Err arm.
    Ok(vec![&call.receiver, &call.args[0]])
}
