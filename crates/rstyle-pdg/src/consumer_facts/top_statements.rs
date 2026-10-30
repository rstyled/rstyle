//! Original direct statement counts, cross-checked against captured block structure.
use super::Availability;
use crate::{ByteSpan, HirFnFacts, HirNode, source_evidence::positions::Positions};
use syn::spanned::Spanned;

#[cfg(test)]
mod tests;

pub(super) fn inspect(facts: &HirFnFacts, function: &syn::ItemFn) -> Availability<usize> {
    if function.sig.asyncness.is_some()
        || !function.attrs.is_empty()
        || function.block.stmts.iter().any(unstable_statement)
    {
        return Availability::Unsupported {
            reason: "Direct statement syntax may change during expansion or lowering".into(),
        };
    }

    let source = facts.source.bytes.as_str();
    let positions = Positions::new(source);
    let block = &facts.body;
    let span = span(&positions, function.block.span());
    if block.kind != "expr_block"
        || block.expansion.is_some()
        || block.span != span
        || crate::source_evidence::byte_range(source, span).ok() != Some(block.snippet.as_str())
        || block.children.len() != function.block.stmts.len()
        || !function
            .block
            .stmts
            .iter()
            .zip(&block.children)
            .all(|(syntax, node)| corresponds(source, &positions, syntax, node))
    {
        return Availability::Unknown {
            reason: "Original direct statements disagree with the captured block roster".into(),
        };
    }
    Availability::Known(function.block.stmts.len())
}

fn unstable_statement(statement: &syn::Stmt) -> bool {
    match statement {
        syn::Stmt::Local(local) => !local.attrs.is_empty(),
        syn::Stmt::Item(item) => {
            matches!(item, syn::Item::Macro(_) | syn::Item::Verbatim(_))
                || !crate::conditional_source::item_attributes(item).is_empty()
        }
        syn::Stmt::Expr(expression, _) => {
            matches!(expression, syn::Expr::Macro(_) | syn::Expr::Verbatim(_))
                || !crate::conditional_source::expression_attributes(expression).is_empty()
        }
        syn::Stmt::Macro(_) => true,
    }
}

fn corresponds(source: &str, positions: &Positions, syntax: &syn::Stmt, node: &HirNode) -> bool {
    let kind_matches = match syntax {
        syn::Stmt::Local(_) => node.kind == "stmt_let",
        syn::Stmt::Item(_) => matches!(
            node.kind.as_str(),
            "stmt_item" | "stmt_size_const" | "stmt_bool_array" | "stmt_const_array_fn"
        ),
        syn::Stmt::Expr(_, Some(_)) => node.kind == "stmt_semi",
        syn::Stmt::Expr(_, None) => node.kind == "stmt_expr" || node.kind.starts_with("expr_"),
        syn::Stmt::Macro(_) => false,
    };
    let span = span(positions, syntax.span());
    kind_matches
        && node.expansion.is_none()
        && node.span == span
        && crate::source_evidence::byte_range(source, span).ok() == Some(node.snippet.as_str())
}

fn span(positions: &Positions, syntax: proc_macro2::Span) -> ByteSpan {
    ByteSpan {
        start: positions.offset(syntax.start()),
        end: positions.offset(syntax.end()),
    }
}
