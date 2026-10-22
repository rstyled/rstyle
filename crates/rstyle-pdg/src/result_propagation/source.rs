//! Original question-mark expression and complete once-evaluated operand.
use crate::{HirNode, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

pub(crate) fn verify(
    node: &HirNode,
    operand: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let original = crate::source_evidence::original_expression(node, facts)?;
    let expression = question_mark(&original)?;
    crate::source_evidence::verify_child_span(node, operand, expression.span(), facts)
}

pub(crate) fn question_mark(expression: &syn::Expr) -> Result<&syn::Expr, String> {
    let syn::Expr::Try(question) = crate::data_aggregates::ungroup(expression) else {
        return Err("Propagation requires an original question-mark expression".into());
    };
    if !question.attrs.is_empty() {
        return Err("Propagation has unknown expression attributes".into());
    }
    Ok(&question.expr)
}
