//! Same-product, same-error extraction; local ownership is proved separately.
mod authority;
pub mod capture;
mod shape;
#[cfg(test)]
mod tests;

use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use capture::Witness;

pub const CAPABILITY: &str = "rust.linear-product-result-flow";

pub struct Checked<'a> {
    pub operand: &'a HirNode,
    pub early_return: &'a HirNode,
    pub input_token: String,
    pub output_token: String,
    pub payload_token: String,
    pub error_token: String,
}

pub fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::gated_node(node, facts, CAPABILITY, |node| {
        node.kind == "expr_try_desugar"
            && node
                .res
                .as_ref()
                .is_some_and(|res| res.kind == capture::RESOLUTION)
    })
}

pub fn verify<'a>(node: &'a HirNode, facts: &dyn EvidenceFacts) -> Result<Checked<'a>, String> {
    if !candidate(node, facts) || node.expansion.is_some() || node.dispatch.is_some() {
        return Err("Product propagation requires its original supported question mark".into());
    }

    let resolution = node.res.as_ref().ok_or("Missing product Result witness")?;
    if resolution.target.is_some() {
        return Err("Product Result witness has an unrelated target".into());
    }

    let witness: Witness =
        serde_json::from_str(&resolution.description).map_err(|e| e.to_string())?;
    if witness.identity != capture::IDENTITY
        || witness.root != node.hir
        || witness.try_span != node.span
    {
        return Err("Product Result witness identifies another expression".into());
    }

    let generated = shape::generated_output(&witness.shape)?;
    let (operand, early_return, residual) =
        crate::scalar_result_propagation::structure_with_generated(node, &witness, &generated)?;

    let boundary = crate::concrete_results::fresh_products::product_boundary(facts)?;
    let input_token = crate::data_aggregates::token(operand)?.to_owned();
    let payload_token = crate::data_aggregates::token(node)?.to_owned();

    if crate::data_types::parse(&input_token)? != boundary.result
        || crate::data_types::parse(&payload_token)? != boundary.ok
        || crate::data_aggregates::parsed_type(residual)? != boundary.result
        || crate::data_aggregates::parsed_type(early_return)? != boundary.result
    {
        return Err(
            "Product propagation requires the exact complete success and error types".into(),
        );
    }

    shape::nominal(&witness.shape, &boundary.ok, &boundary.error)?;
    authority::verify(&witness, operand, facts)?;
    crate::result_propagation::source::verify(node, operand, facts)?;

    Ok(Checked {
        operand,
        early_return,
        input_token,
        output_token: boundary.result.text()?,
        payload_token,
        error_token: boundary.error.text()?,
    })
}
