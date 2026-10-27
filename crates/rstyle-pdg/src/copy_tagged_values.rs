//! Values at the two original, checked Copy conversion boundaries.
mod activation;
pub(crate) mod call_wrappers;
pub(crate) use activation::{body_operation, candidate, permits_type, verify_function};
mod matched;
pub(crate) mod origins;
mod source;
#[cfg(test)]
mod tests;

use crate::nominal_boundary::resolution::CheckedCopyConversion;
use crate::{HirNode, concrete_types::TypeTree, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

pub(crate) const CAPABILITY: &str = "rust.copy-tagged-conversion-values";

fn typed(node: &HirNode, tree: &TypeTree) -> Result<(), String> {
    let expected = crate::data_types::TypeToken::from_tree(tree).text()?;
    if node.expansion.is_some() || crate::data_aggregates::token(node)? != expected {
        return Err("Copy conversion HIR type or source provenance disagrees".into());
    }
    Ok(())
}

pub(crate) struct Match<'a> {
    pub scrutinee: &'a HirNode,
    pub pattern: &'a HirNode,
    pub binding: &'a HirNode,
    pub success: &'a HirNode,
    pub wildcard: &'a HirNode,
    pub failure: &'a HirNode,
    pub tag: String,
}

struct Context<'a> {
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
    endpoints: Endpoints<'a>,
}

struct Endpoints<'a> {
    output: &'a TypeTree,
    product: &'a TypeTree,
    tagged: &'a TypeTree,
    error: Option<&'a TypeTree>,
}

impl<'a> Endpoints<'a> {
    fn checked(receipt: &'a CheckedCopyConversion) -> Self {
        Self {
            output: receipt.output(),
            product: receipt.product(),
            tagged: receipt.tagged(),
            error: receipt.error(),
        }
    }
}

pub(crate) fn expression(facts: &dyn EvidenceFacts) -> Result<&HirNode, String> {
    let [node] = facts.body.children.as_slice() else {
        return Err("Copy conversion requires its exact single-expression body".into());
    };
    if facts.body.kind != "expr_block" || facts.body.expansion.is_some() {
        return Err("Copy conversion requires its original function body".into());
    }
    Ok(node)
}

pub(crate) fn constructor(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    checked: &CheckedCopyConversion,
) -> Result<String, String> {
    let context = Context {
        node,
        facts,
        endpoints: Endpoints::checked(checked),
    };
    constructor_context(node, &context)
}

fn constructor_context(node: &HirNode, context: &Context<'_>) -> Result<String, String> {
    if context.endpoints.error.is_some() || node.kind != "expr_call" || node.children.len() != 1 {
        return Err("Copy conversion requires its original wrapping constructor".into());
    }
    let syntax = crate::source_evidence::original_expression(node, context.facts)?;
    let syn::Expr::Call(call) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Copy conversion constructor lost its call syntax".into());
    };
    if call.args.len() != 1 {
        return Err("Copy conversion constructor requires one saved operand".into());
    }

    typed(node, context.endpoints.tagged)?;
    typed(&node.children[0], context.endpoints.product)?;
    context.correspondence(call.args[0].span(), &node.children[0])?;
    context.formal(&node.children[0])?;
    let tag = context.variant(node, "resolved_callee")?;
    source::dispatch(node)?;
    Ok(tag)
}

pub(crate) fn matched<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
    checked: &CheckedCopyConversion,
) -> Result<Match<'a>, String> {
    matched::verify(
        node,
        &Context {
            node,
            facts,
            endpoints: Endpoints::checked(checked),
        },
    )
}
