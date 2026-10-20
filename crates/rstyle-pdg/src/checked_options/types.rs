//! Exact concrete signatures; source names cannot stand in for instantiated types.

use super::*;
use crate::{
    concrete_types::{Observation, TypeTree},
    data_receivers::Kind,
    data_types::TypeToken,
};

pub(super) fn payload(node: &HirNode) -> Result<String, String> {
    let token = crate::data_aggregates::token(node)?;
    let value: TypeToken = serde_json::from_str(token).map_err(|error| error.to_string())?;
    let TypeToken::Option(payload) = &value else {
        return Err("Expected a concrete core Option observation".into());
    };
    let TypeToken::Shared(scalar) = payload.as_ref() else {
        return Err("Checked Option requires a shared scalar payload".into());
    };
    let TypeToken::Scalar(scalar) = scalar.as_ref() else {
        return Err("Checked Option requires a scalar referent".into());
    };

    let tree = result(scalar);
    crate::concrete_types::validate(&tree)?;
    if crate::data_types::token(&tree)? != token {
        return Err("Checked Option type observation is not canonical".into());
    }
    Ok(scalar.clone())
}

fn result(scalar: &str) -> TypeTree {
    TypeTree::Option(Box::new(TypeTree::Shared(Box::new(TypeTree::Scalar(
        scalar.into(),
    )))))
}

pub(super) fn get(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<String, String> {
    let scalar = payload(node)?;
    let receiver = TypeTree::Shared(Box::new(TypeTree::Slice(Box::new(TypeTree::Scalar(
        scalar.clone(),
    )))));
    let expected = [crate::data_types::token(&receiver)?, "usize".into()];
    let operands = node
        .children
        .iter()
        .map(|child| crate::data_aggregates::token(child).map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing checked-get dispatch")?;
    if dispatch.kind != "resolved_method"
        || dispatch.op != GET
        || dispatch.operand_tys != expected
        || operands != expected
        || dispatch.result_ty != crate::data_types::token(&result(&scalar))?
    {
        return Err("Checked get requires exact shared-slice/usize/Option types".into());
    }
    receiver_signature(node, facts, receiver, &scalar)?;

    Ok(scalar)
}

fn receiver_signature(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    receiver: TypeTree,
    scalar: &str,
) -> Result<(), String> {
    let capture = crate::data_receivers::verify(facts)?;
    let record = capture
        .methods
        .iter()
        .find(|record| record.call == node.hir)
        .ok_or("Missing checked-get receiver observation")?;
    let observed = |ty| Observation::Captured { ty };
    let kinds: Vec<_> = record.adjustments.iter().map(|step| &step.kind).collect();

    if record.receiver_type != observed(receiver.clone())
        || record.adjusted_type != observed(receiver.clone())
        || record.signature_inputs
            != [
                observed(receiver),
                observed(TypeTree::Scalar("usize".into())),
            ]
        || record.signature_result != observed(result(scalar))
        || !(kinds.is_empty() || kinds == [&Kind::BuiltinDereference, &Kind::SharedBorrow])
    {
        return Err("Checked get lacks its exact immutable instantiated signature".into());
    }
    Ok(())
}
