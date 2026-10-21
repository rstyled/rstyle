use super::*;
use crate::concrete_types::Observation;

pub(super) fn verify(node: &HirNode, kind: Kind, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let (inputs, output) = kind.signature()?;
    let dispatch = node.dispatch.as_ref().ok_or("Missing integer dispatch")?;
    let direct = kind.implementation().is_some();
    let operands = node
        .children
        .iter()
        .map(crate::data_aggregates::token)
        .collect::<Result<Vec<_>, _>>()?;

    if node.expansion.is_some()
        || dispatch.kind
            != if direct {
                "resolved_direct"
            } else {
                "resolved_method"
            }
        || node.kind
            != if direct {
                "expr_call"
            } else {
                "expr_method_call"
            }
        || dispatch.operand_tys != inputs
        || dispatch.result_ty != output
        || crate::data_aggregates::token(node)? != output
        || operands != inputs
    {
        return Err("Integer call loses its exact builtin signature".into());
    }
    if !direct {
        receiver(node, kind, facts)?;
    }
    Ok(())
}

fn receiver(node: &HirNode, kind: Kind, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let capture = crate::data_receivers::verify(facts)?;
    let method = capture
        .methods
        .iter()
        .find(|method| method.call == node.hir)
        .ok_or("Missing captured integer method receiver")?;
    let (inputs, output) = kind.trees();
    let observed = |ty| Observation::Captured { ty };
    let receiver = observed(inputs[0].clone());

    if method.receiver_type != receiver
        || method.adjusted_type != receiver
        || method.signature_inputs != inputs.into_iter().map(observed).collect::<Vec<_>>()
        || method.signature_result != observed(output)
        || !method.adjustments.is_empty()
    {
        return Err("Integer method receiver/signature disagrees with capture".into());
    }
    Ok(())
}
