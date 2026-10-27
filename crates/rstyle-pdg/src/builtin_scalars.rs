//! Exact primitive operand and builtin dispatch agreement for scalar operations.
use crate::{HirNode, frontend::normalized};

pub(crate) fn matches(node: &HirNode, operands: &[&str], result: &str, operators: &[&str]) -> bool {
    node.children.len() == operands.len()
        && node
            .children
            .iter()
            .zip(operands)
            .all(|(child, ty)| normalized(child) == Some(*ty))
        && normalized(node) == Some(result)
        && node.dispatch.as_ref().is_some_and(|dispatch| {
            dispatch.kind == "builtin"
                && operators.contains(&dispatch.op.as_str())
                && dispatch.operand_tys == operands
                && dispatch.result_ty == result
        })
}
