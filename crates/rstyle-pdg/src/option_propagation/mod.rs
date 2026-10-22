//! Shared scalar Option propagation with exact compiler/core evidence.
mod authority;
pub mod capture;
mod source;
#[cfg(test)]
mod tests;

use crate::result_propagation::{arm, local_use, observed, only_child, raw_call};
use crate::{HirNode, unit_store::EvidenceFacts};
use capture::Witness;

pub const CAPABILITY: &str = "rust.concrete-option-propagation";

pub struct Checked<'a> {
    pub operand: &'a HirNode,
    pub early_return: &'a HirNode,
    pub input_token: String,
    pub output_token: String,
    pub payload_token: String,
}

pub fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::gated_node(node, facts, CAPABILITY, |node| {
        node.kind == "expr_try_desugar"
            && node
                .res
                .as_ref()
                .is_some_and(|resolution| resolution.kind == capture::RESOLUTION)
    })
}

pub fn verify<'a>(node: &'a HirNode, facts: &dyn EvidenceFacts) -> Result<Checked<'a>, String> {
    if !candidate(node, facts) || node.expansion.is_some() || node.dispatch.is_some() {
        return Err("Option propagation requires its original supported question mark".into());
    }
    let witness = witness(node)?;

    let (operand, early_return) = structure(node, &witness)?;
    let input_token = crate::data_aggregates::token(operand)?.to_owned();
    let output_token = facts
        .signature
        .return_ty
        .normalized
        .as_deref()
        .ok_or("Missing complete Option return observation")?
        .to_owned();
    let crate::data_types::TypeToken::Option(payload) = crate::data_types::parse(&input_token)?
    else {
        return Err("Option propagation input is not core Option".into());
    };
    if !matches!(payload.as_ref(), crate::data_types::TypeToken::Shared(inner) if matches!(inner.as_ref(), crate::data_types::TypeToken::Scalar(name) if name == "u8"))
    {
        return Err("Option propagation requires a shared u8 payload".into());
    }
    let payload_token = payload.text()?;
    let output_value = early_return
        .children
        .first()
        .ok_or("Missing Option residual conversion")?;
    crate::checked_option_products::payload(output_value, facts)?;
    if !matches!(
        crate::data_types::parse(&output_token)?,
        crate::data_types::TypeToken::Option(_)
    ) || node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(payload_token.as_str())
        || output_value
            .ty
            .as_ref()
            .and_then(|ty| ty.normalized.as_deref())
            != Some(output_token.as_str())
    {
        return Err("Option propagation loses its payload or complete output Option".into());
    }
    authority::verify(&witness, operand, facts)?;
    source::verify(node, operand, facts)?;
    Ok(Checked {
        operand,
        early_return,
        input_token,
        output_token,
        payload_token,
    })
}

fn witness(node: &HirNode) -> Result<Witness, String> {
    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing complete Option propagation witness")?;
    if resolution.kind != capture::RESOLUTION || resolution.target.is_some() {
        return Err("Option propagation has an unrelated resolution record".into());
    }
    let witness: Witness =
        serde_json::from_str(&resolution.description).map_err(|e| e.to_string())?;
    if witness.identity != capture::IDENTITY
        || witness.root != node.hir
        || witness.try_span != node.span
    {
        return Err("Option propagation witness identifies another expression".into());
    }

    Ok(witness)
}

fn structure<'a>(
    node: &'a HirNode,
    witness: &Witness,
) -> Result<(&'a HirNode, &'a HirNode), String> {
    let [branch, broken, continued] = node.children.as_slice() else {
        return Err("Option propagation requires exactly two complete generated arms".into());
    };
    let operand = only_child(branch, "expr_call", &witness.branch_call)?;
    if operand.hir != witness.operand || operand.span != witness.operand_span {
        return Err(
            "Propagation operand identity/range disagrees with the compiler witness".into(),
        );
    }

    let early_return = arm(broken, &witness.break_arm)?;
    let residual = only_child(early_return, "expr_ret", &witness.early_return)?;
    let usage = only_child(residual, "expr_call", &witness.residual_call)?;

    raw_call(branch, &witness.branch)?;
    raw_call(residual, &witness.residual)?;
    observed(branch, &witness.shape.control_flow)?;
    observed(usage, &witness.shape.residual)?;

    local_use(usage, &witness.break_arm)?;
    let payload_use = arm(continued, &witness.continue_arm)?;
    local_use(payload_use, &witness.continue_arm)?;
    if payload_use.ty.as_ref().map(|ty| ty.observed.as_str())
        != Some(witness.shape.payload.as_str())
    {
        return Err(
            "Propagation success continuation must yield its exact shared scalar payload".into(),
        );
    }

    if witness.break_arm.binding == witness.continue_arm.binding
        || witness.break_arm.binding.owner != node.hir.owner
        || witness.continue_arm.binding.owner != node.hir.owner
    {
        return Err(
            "Propagation continuation bindings are not distinct owner-local identities".into(),
        );
    }
    Ok((operand, early_return))
}
