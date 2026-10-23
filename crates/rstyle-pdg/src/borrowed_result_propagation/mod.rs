//! Exact borrowed success extraction and differently typed same-error return.
mod authority;
pub mod capture;
#[cfg(test)]
mod tests;

use crate::result_propagation::{arm, local_use, observed, only_child, raw_call};
use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use capture::Witness;

pub const CAPABILITY: &str = "rust.checked-borrowed-result-flow";

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
        return Err(
            "Borrowed Result propagation requires its original supported question mark".into(),
        );
    }

    let resolution = node.res.as_ref().ok_or("Missing borrowed Result witness")?;

    if resolution.target.is_some() {
        return Err("Borrowed Result witness has an unrelated target".into());
    }

    let witness: Witness =
        serde_json::from_str(&resolution.description).map_err(|e| e.to_string())?;

    if witness.identity != capture::IDENTITY
        || witness.root != node.hir
        || witness.try_span != node.span
    {
        return Err("Borrowed Result witness identifies another expression".into());
    }

    let (operand, early_return, residual) = structure(node, &witness)?;

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

    if crate::data_aggregates::token(residual)? != output_token {
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
        return Err("Borrowed propagation operand is not Result".into());
    };

    let TypeToken::Result {
        ok: output_ok,
        err: output_err,
    } = output
    else {
        return Err("Borrowed propagation return is not Result".into());
    };

    if *ok != payload
        || !crate::borrowed_result_types::GRAMMAR.permits(&ok, &err)
        || !crate::borrowed_result_types::GRAMMAR.permits(&output_ok, &output_err)
        || err != output_err
        || ok == output_ok
        || !matches!(ok.as_ref(), TypeToken::Shared(inner) if matches!(inner.as_ref(), TypeToken::Slice(element) if matches!(element.as_ref(), TypeToken::Scalar(name) if name == "u8")))
        || !matches!(output_ok.as_ref(), TypeToken::Tuple(values) if values.len() == 2)
    {
        return Err(
            "Borrowed propagation requires exact slice input, product output and identical error"
                .into(),
        );
    }

    let nominal =
        crate::borrowed_result_types::error(&err).ok_or("Unsupported complete error type")?;

    if !facts.nominal_resolution()?.checked_no_drop(nominal) {
        return Err(
            "Borrowed propagation lacks separately checked complete no-drop error evidence".into(),
        );
    }
    err.text()
}

fn structure<'a>(
    node: &'a HirNode,
    witness: &Witness,
) -> Result<(&'a HirNode, &'a HirNode, &'a HirNode), String> {
    let [branch, broken, continued] = node.children.as_slice() else {
        return Err("Borrowed propagation requires two complete generated arms".into());
    };

    let operand = only_child(branch, "expr_call", &witness.branch_call)?;

    if operand.hir != witness.operand || operand.span != witness.operand_span {
        return Err("Borrowed propagation operand identity/range disagrees".into());
    }

    let early_return = arm(broken, &witness.break_arm)?;

    let residual = only_child(early_return, "expr_ret", &witness.early_return)?;

    let usage = only_child(residual, "expr_call", &witness.residual_call)?;

    raw_call(branch, &witness.branch)?;

    raw_call(residual, &witness.residual)?;

    observed(branch, &witness.shape.control_flow)?;

    observed(usage, &witness.shape.residual)?;

    observed(residual, &authority::generated_output(&witness.shape)?)?;

    local_use(usage, &witness.break_arm)?;

    let success = arm(continued, &witness.continue_arm)?;

    local_use(success, &witness.continue_arm)?;

    observed(success, &witness.shape.payload)?;

    if success.ty.as_ref().and_then(|ty| ty.normalized.as_deref())
        != node.ty.as_ref().and_then(|ty| ty.normalized.as_deref())
        || witness.break_arm.binding == witness.continue_arm.binding
        || witness.break_arm.binding.owner != node.hir.owner
        || witness.continue_arm.binding.owner != node.hir.owner
    {
        return Err(
            "Borrowed propagation requires exact distinct owner-local payload bindings".into(),
        );
    }
    Ok((operand, early_return, residual))
}
