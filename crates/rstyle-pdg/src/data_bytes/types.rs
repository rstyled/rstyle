//! Concrete pinned slice/result/array signatures and receiver adjustments.

use super::*;
use crate::concrete_types::{Observation, TypeTree};

pub(super) fn array() -> TypeTree {
    TypeTree::Array {
        element: Box::new(TypeTree::Scalar("u8".into())),
        length: 4,
    }
}

fn result() -> TypeTree {
    TypeTree::SliceArrayResult {
        length: 4,
        borrowed: false,
    }
}

pub(super) fn verify(node: &HirNode, kind: Kind, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let (input, output) = match kind {
        Kind::Copy => (
            TypeTree::Shared(Box::new(TypeTree::Slice(Box::new(TypeTree::Scalar(
                "u8".into(),
            ))))),
            result(),
        ),
        Kind::Unwrap => (result(), array()),
        Kind::Native => (array(), TypeTree::Scalar("u32".into())),
    };
    let operand = node.children.first().ok_or("Missing byte-call operand")?;
    let dispatch = node.dispatch.as_ref().ok_or("Missing byte-call dispatch")?;
    if dispatch.kind
        != if kind == Kind::Native {
            "resolved_direct"
        } else {
            "resolved_method"
        }
        || dispatch.operand_tys != [crate::data_aggregates::token(operand)?]
        || dispatch.result_ty != crate::data_types::token(&output)?
        || crate::data_aggregates::token(node)? != dispatch.result_ty
    {
        return Err("Byte-call dispatch and typed operands disagree".into());
    }
    if kind == Kind::Native {
        if crate::data_aggregates::token(operand)? != crate::data_types::token(&input)? {
            return Err("Native byte conversion requires exactly [u8; 4]".into());
        }
        return Ok(());
    }

    verify_receiver(node, kind, facts, Signature { input, output })
}

struct Signature {
    input: TypeTree,
    output: TypeTree,
}

fn verify_receiver(
    node: &HirNode,
    kind: Kind,
    facts: &dyn EvidenceFacts,
    signature: Signature,
) -> Result<(), String> {
    let capture = crate::data_receivers::verify(facts)?;
    let receiver = capture
        .methods
        .iter()
        .find(|record| record.call == node.hir)
        .ok_or("Missing byte-call receiver")?;
    let observation = |ty| Observation::Captured { ty };
    if receiver.adjusted_type != observation(signature.input.clone())
        || receiver.signature_inputs != [observation(signature.input)]
        || receiver.signature_result != observation(signature.output)
    {
        return Err("Byte-call instantiated signature disagrees".into());
    }
    let allowed = match kind {
        Kind::Copy => receiver.adjustments.iter().all(|step| {
            matches!(
                step.kind,
                crate::data_receivers::Kind::SharedBorrow
                    | crate::data_receivers::Kind::BuiltinDereference
            )
        }),
        Kind::Unwrap => receiver.adjustments.is_empty(),
        Kind::Native => unreachable!(),
    };
    if !allowed {
        return Err("Byte call requires builtin shared receiver adjustments".into());
    }

    Ok(())
}
