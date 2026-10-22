//! Same-error `Result<(), E>` propagation with exact compiler/core evidence.
pub(crate) mod authority;
pub mod capture;
pub(crate) mod source;
#[cfg(test)]
mod tests;

use crate::{HirNode, unit_store::EvidenceFacts};
use capture::Witness;

pub const CAPABILITY: &str = "rust.same-error-result-propagation";

pub struct Checked<'a> {
    pub operand: &'a HirNode,
    pub early_return: &'a HirNode,
    pub result_token: String,
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
        return Err("Result propagation requires its original supported question mark".into());
    }
    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing complete Result propagation witness")?;
    if resolution.kind != capture::RESOLUTION || resolution.target.is_some() {
        return Err("Result propagation has an unrelated resolution record".into());
    }
    let witness: Witness =
        serde_json::from_str(&resolution.description).map_err(|e| e.to_string())?;
    if witness.identity != capture::IDENTITY
        || witness.root != node.hir
        || witness.try_span != node.span
    {
        return Err("Result propagation witness identifies another expression".into());
    }

    let (operand, early_return) = structure(node, &witness)?;
    crate::concrete_results::payload(operand, facts)?;
    let result_token = crate::data_aggregates::token(operand)?.to_owned();
    if facts.signature.return_ty.normalized.as_deref() != Some(result_token.as_str())
        || node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some("unit")
        || early_return
            .ty
            .as_ref()
            .and_then(|ty| ty.normalized.as_deref())
            != Some(result_token.as_str())
    {
        return Err(
            "Propagation requires identical complete input/output Results and unit success".into(),
        );
    }
    authority::verify(&witness, operand, facts)?;
    source::verify(node, operand, facts)?;
    Ok(Checked {
        operand,
        early_return,
        result_token,
    })
}

fn structure<'a>(
    node: &'a HirNode,
    witness: &Witness,
) -> Result<(&'a HirNode, &'a HirNode), String> {
    let [branch, broken, continued] = node.children.as_slice() else {
        return Err("Result propagation requires exactly two complete generated arms".into());
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
    let unit = arm(continued, &witness.continue_arm)?;
    local_use(unit, &witness.continue_arm)?;
    if unit.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some("unit") {
        return Err("Propagation success continuation must yield its exact unit payload".into());
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

pub(crate) fn only_child<'a>(
    node: &'a HirNode,
    kind: &str,
    identity: &crate::HirRef,
) -> Result<&'a HirNode, String> {
    let [child] = node.children.as_slice() else {
        return Err("Propagation generated operation has missing or extra operands".into());
    };
    if node.kind != kind || &node.hir != identity || node.is_unsafe_block {
        return Err(
            "Propagation generated operation disagrees with the complete compiler witness".into(),
        );
    }
    Ok(child)
}

pub(crate) fn arm<'a>(node: &'a HirNode, witness: &capture::Arm) -> Result<&'a HirNode, String> {
    let [pattern, body] = node.children.as_slice() else {
        return Err("Propagation arm has a guard or an incomplete body".into());
    };
    if node.kind != "match_arm"
        || node.hir != witness.arm
        || pattern.kind != "pat_unsupported"
        || pattern.hir != witness.pattern
        || !pattern.children.is_empty()
        || pattern.binding.is_some()
    {
        return Err("Propagation arm loses its exact captured variant/payload pattern".into());
    }
    Ok(body)
}

pub(crate) fn local_use(node: &HirNode, witness: &capture::Arm) -> Result<(), String> {
    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing generated continuation use")?;
    if node.kind != "expr_path"
        || node.hir != witness.usage
        || !node.children.is_empty()
        || resolution.kind != "local"
        || resolution.target != Some(witness.binding.clone())
        || node.dispatch.is_some()
        || node.binding.is_some()
    {
        return Err("Propagation generated use is not its exact immutable payload binding".into());
    }
    Ok(())
}

pub(crate) fn raw_call(node: &HirNode, selection: &capture::Selection) -> Result<(), String> {
    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing retained generated core callee")?;
    let method: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|e| e.to_string())?;
    if resolution.kind != "resolved_callee"
        || resolution.target.is_some()
        || method != selection.method
        || node.binding.is_some()
        || node.lit.is_some()
    {
        return Err(
            "Retained propagation call disagrees with selected implementation evidence".into(),
        );
    }

    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing retained propagation dispatch")?;
    if dispatch.kind != "resolved_direct"
        || dispatch.op != method.path
        || dispatch.operand_tys != [selection.input.clone()]
        || (dispatch.result_ty != selection.output
            && node.ty.as_ref().and_then(|ty| ty.normalized.as_deref())
                != Some(dispatch.result_ty.as_str()))
    {
        return Err("Retained propagation call has unrelated dispatch or operands".into());
    }
    Ok(())
}

pub(crate) fn observed(node: &HirNode, expected: &str) -> Result<(), String> {
    if node.ty.as_ref().map(|ty| ty.observed.as_str()) != Some(expected) {
        return Err(
            "Generated propagation type disagrees with its exact concrete substitution".into(),
        );
    }
    Ok(())
}
