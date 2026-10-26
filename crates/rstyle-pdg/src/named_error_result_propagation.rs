//! Exact unit-to-reader propagation with independently no-drop named errors.
mod authority;
pub mod capture;
#[cfg(test)]
mod tests;

use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use capture::Witness;

pub const CAPABILITY: &str = "rust.named-error-reader-flow";

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
    let witness = checked_witness(node, facts)?;

    authority::concrete_shape(&witness.shape)?;
    let (operand, early_return, residual) =
        crate::scalar_result_propagation::structure_with_generated(
            node,
            &witness,
            &witness.shape.output,
        )?;

    let input_token = crate::data_aggregates::token(operand)?.to_owned();

    let output_token = facts
        .signature
        .return_ty
        .normalized
        .as_ref()
        .ok_or("Missing complete Result return type")?
        .clone();

    let payload_token = crate::data_aggregates::token(node)?.to_owned();

    let error_token = types(&input_token, &output_token, &payload_token, facts)?;

    if crate::data_aggregates::token(residual)? != output_token
        || crate::data_aggregates::token(early_return)? != output_token
    {
        return Err("Residual conversion must construct the complete output Result".into());
    }

    authority::verify(&witness, operand, facts)?;

    crate::result_propagation::source::verify(node, operand, facts)?;

    Ok(Checked {
        operand,
        early_return,
        input_token,
        output_token,
        payload_token,
        error_token,
    })
}

fn checked_witness(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<Witness, String> {
    if !candidate(node, facts) || node.expansion.is_some() || node.dispatch.is_some() {
        return Err(
            "Named error reader Result propagation requires its original supported question mark"
                .into(),
        );
    }

    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing named error reader witness")?;

    if resolution.target.is_some() {
        return Err("Named error reader Result witness has an unrelated target".into());
    }

    let witness: Witness =
        serde_json::from_str(&resolution.description).map_err(|e| e.to_string())?;

    if witness.identity != capture::IDENTITY
        || witness.root != node.hir
        || witness.try_span != node.span
    {
        return Err("Named error reader Result witness identifies another expression".into());
    }

    Ok(witness)
}

fn types(
    input: &str,
    output: &str,
    payload: &str,
    facts: &dyn EvidenceFacts,
) -> Result<String, String> {
    let input = crate::data_types::parse(input)?;

    let output = crate::data_types::parse(output)?;

    let payload = crate::data_types::parse(payload)?;

    let TypeToken::Result { ok, err } = input else {
        return Err("Named error reader propagation operand is not Result".into());
    };

    let TypeToken::Result {
        ok: output_ok,
        err: output_err,
    } = output
    else {
        return Err("Named error reader propagation return is not Result".into());
    };

    if *ok != payload
        || *ok != TypeToken::Scalar("unit".into())
        || !reader_tuple(&output_ok)
        || err != output_err
    {
        return Err(
            "Reader propagation requires unit input, exact (u32, usize) output and identical error"
                .into(),
        );
    }
    let error = crate::named_error_payloads::error_tree(facts)?
        .ok_or("Reader propagation lacks independently checked named error ownership")?;
    if TypeToken::from_tree(&error) != *err {
        return Err("Reader propagation error differs from its complete no-drop boundary".into());
    }
    err.text()
}

pub(crate) fn success(token: &TypeToken) -> bool {
    matches!(token, TypeToken::Scalar(value) if value == "unit") || reader_tuple(token)
}

fn reader_tuple(token: &TypeToken) -> bool {
    matches!(token, TypeToken::Tuple(values) if matches!(values.as_slice(),
        [TypeToken::Scalar(first), TypeToken::Scalar(second)]
        if first == "u32" && second == "usize"))
}
