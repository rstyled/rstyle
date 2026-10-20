use super::*;
use crate::concrete_types::{Observation, TypeTree};

pub(super) fn verify(
    node: &HirNode,
    operand: &HirNode,
    kind: Kind,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let (input, output) = kind.signature()?;
    let dispatch = node.dispatch.as_ref().ok_or("Missing integer dispatch")?;

    if node.expansion.is_some()
        || dispatch.kind
            != if kind == Kind::Convert {
                "resolved_direct"
            } else {
                "resolved_method"
            }
        || node.kind
            != if kind == Kind::Convert {
                "expr_call"
            } else {
                "expr_method_call"
            }
        || dispatch.operand_tys != [input.clone()]
        || dispatch.result_ty != output
        || crate::data_aggregates::token(node)? != output
        || crate::data_aggregates::token(operand)? != input
    {
        return Err("Integer call loses its exact builtin signature".into());
    }
    if kind != Kind::Convert {
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
    let (input, output) = if kind == Kind::Count {
        (
            TypeTree::Scalar("u32".into()),
            TypeTree::Scalar("u32".into()),
        )
    } else {
        (
            TypeTree::IntConversionResult("usize".into()),
            TypeTree::Scalar("usize".into()),
        )
    };
    let observed = |ty| Observation::Captured { ty };

    if method.receiver_type != observed(input.clone())
        || method.adjusted_type != observed(input.clone())
        || method.signature_inputs != [observed(input)]
        || method.signature_result != observed(output)
        || !method.adjustments.is_empty()
    {
        return Err("Integer method receiver/signature disagrees with capture".into());
    }
    Ok(())
}
