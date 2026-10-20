//! Original index expression and evaluated operand correspondence.

use crate::{HirNode, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

pub(super) fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let bytes = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original index source")?;
    let expression: syn::Expr = syn::parse_str(bytes).map_err(|error| error.to_string())?;
    let syn::Expr::Index(expression) = crate::data_aggregates::ungroup(&expression) else {
        return Err("Indexing lacks original index syntax".into());
    };
    let [base, index] = node.children.as_slice() else {
        return Err("Indexing lacks original operand correspondence".into());
    };

    for (expression, operand) in [(&expression.expr, base), (&expression.index, index)] {
        let span = crate::data_aggregates::ungroup(expression).span();
        let syntax = crate::ByteSpan {
            start: node.span.start
                + crate::source_evidence::byte_offset(bytes, span.start()) as u32,
            end: node.span.start + crate::source_evidence::byte_offset(bytes, span.end()) as u32,
        };
        if !crate::source_evidence::corresponding_span(&facts.source.bytes, syntax, operand.span) {
            return Err("Indexing loses its exact base/index source positions".into());
        }
    }
    Ok(())
}
