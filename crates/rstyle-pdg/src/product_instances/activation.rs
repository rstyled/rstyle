//! Only an owning private projection authorizes the complete Span identity call.
use super::{CheckedProjection, SpanArgument};
use crate::source_evidence::checked_activation::deferred_call as findings;
use crate::{HirNode, call_summaries::Summary, unit_store::EvidenceFacts};

#[cfg(test)]
use crate::HirFnFacts;
#[cfg(test)]
mod tests;

pub const CAPABILITY: &str = "rust.checked-product-constructor-instances";
pub const CONSTRUCT: &str = "data.product-instance.identity-into";
pub const SUMMARY: &str = "rstyle.product-instance.span-identity";

pub fn verify(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    crate::source_evidence::checked_activation::presence(facts, checked)
}

pub fn source_findings(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    verify(facts)
}

fn checked(facts: &dyn EvidenceFacts) -> Result<Option<&CheckedProjection>, String> {
    let Some(checked) = facts.product_instance() else {
        return Ok(None);
    };
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || checked.argument() != SpanArgument::Span
    {
        return Err(
            "Product identity requires its checked Span substitution and capability".into(),
        );
    }

    if checked.facts() != facts.facts() {
        return Err("Checked product instance differs from its immutable owning facts".into());
    }
    findings(facts.facts(), checked.operation())?;
    Ok(Some(checked))
}

pub fn operation(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = checked(facts)? else {
        return Ok(false);
    };
    checked_operation(node, checked.operation())
}

fn checked_operation(node: &HirNode, expected: &HirNode) -> Result<bool, String> {
    if node.hir != expected.hir {
        return Ok(false);
    }
    if node != expected || node.kind != "expr_method_call" || node.children.len() != 1 {
        return Err("Product identity changes its exact checked operation or sole receiver".into());
    }
    Ok(true)
}

pub fn summary(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<Option<Summary>, String> {
    if !operation(node, facts)? {
        return Ok(None);
    }
    identity_summary(node).map(Some)
}

fn identity_summary(node: &HirNode) -> Result<Summary, String> {
    let [receiver] = node.children.as_slice() else {
        return Err("Product identity summary requires one whole-value operand".into());
    };
    let input = crate::data_aggregates::token(receiver)?;
    let output = crate::data_aggregates::token(node)?;
    let input_type = crate::data_types::parse(input)?;
    if input != output
        || !matches!(input_type, crate::data_types::TypeToken::Struct { .. })
        || !crate::data_aggregates::reference_free(&input_type)
    {
        return Err(
            "Product identity must preserve its entire reference-free nominal value".into(),
        );
    }

    Ok(Summary {
        id: SUMMARY.into(),
        version: "1.0.0".into(),
        callee_path: "core::convert::Into::into".into(),
        callee_crate: "core".into(),
        argument_types: vec![input.into()],
        result_type: output.into(),
        memory_effects: false,
        mutates_arguments: false,
        may_panic: false,
        always_returns: true,
        overflow_behavior: "not-applicable".into(),
    })
}
