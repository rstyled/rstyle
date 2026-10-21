//! Original operators and operand source spans independently constrain HIR.
use crate::{HirNode, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

pub(super) fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !matches!(
        node.kind.as_str(),
        "expr_binary" | "expr_unary" | "expr_cast"
    ) {
        return Ok(());
    }

    let expression = crate::source_evidence::original_expression(node, facts)?;
    let expression = crate::data_aggregates::ungroup(&expression);
    let (operator, operands) = parts(expression)?;
    if node.dispatch.as_ref().map(|dispatch| dispatch.op.as_str()) != Some(operator)
        || operands.len() != node.children.len()
    {
        return Err("Float operator/operand inventory disagrees with original syntax".into());
    }

    let bytes = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Float source operation span is outside its function")?;
    for (original, captured) in operands.into_iter().zip(&node.children) {
        let span = crate::data_aggregates::ungroup(original).span();
        let expected = crate::ByteSpan {
            start: node.span.start
                + crate::source_evidence::byte_offset(bytes, span.start()) as u32,
            end: node.span.start + crate::source_evidence::byte_offset(bytes, span.end()) as u32,
        };
        if !crate::source_evidence::corresponding_span(&facts.source.bytes, expected, captured.span)
        {
            return Err("Float operation loses ordered original operand correspondence".into());
        }
    }
    Ok(())
}

fn parts(expression: &syn::Expr) -> Result<(&'static str, Vec<&syn::Expr>), String> {
    match expression {
        syn::Expr::Binary(value) => {
            let operator = match value.op {
                syn::BinOp::Add(_) => "+",
                syn::BinOp::Mul(_) => "*",
                _ => {
                    return Err(
                        "Float source operator is outside ordered add/multiply support".into(),
                    );
                }
            };
            Ok((operator, vec![&value.left, &value.right]))
        }
        syn::Expr::Unary(value) if matches!(value.op, syn::UnOp::Neg(_)) => {
            Ok(("-", vec![&value.expr]))
        }
        syn::Expr::Cast(value) => {
            if !matches!(&*value.ty, syn::Type::Path(ty) if ty.qself.is_none() && ty.path.is_ident("f32"))
            {
                return Err("Float source cast requires the original primitive f32 target".into());
            }
            Ok(("as", vec![&value.expr]))
        }
        _ => Err("Float HIR operation disagrees with original expression kind".into()),
    }
}
