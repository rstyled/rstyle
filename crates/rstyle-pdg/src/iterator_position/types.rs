use super::*;
use crate::{
    concrete_types::{Observation, TypeTree},
    data_receivers::Kind,
};

pub(super) fn verify(
    node: &HirNode,
    iterator: &HirNode,
    receiver: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    if !iterator_candidate(iterator, facts) || iterator.expansion.is_some() {
        return Err("Byte position requires one original fresh slice iterator".into());
    }
    let registry =
        crate::data_library::registry_for(facts).ok_or("Unpinned byte iterator build")?;
    let callee = crate::data_library::callee(node)?;
    let selected = callee
        .implementation
        .as_deref()
        .ok_or("Missing selected slice position implementation")?;
    if callee.crate_name != "core"
        || callee.crate_hash != registry.core_hash
        || registry.definitions.get("core::iter::Iterator::position")
            != Some(&callee.definition_hash)
        || selected.crate_name != "core"
        || selected.crate_hash != registry.core_hash
        || registry.definitions.get("core::slice::Iter::position")
            != Some(&selected.definition_hash)
        || !matches!(
            selected.path.as_str(),
            "<std::slice::Iter<'a, T> as std::iter::Iterator>::position"
                | "<core::slice::Iter<'a, T> as core::iter::Iterator>::position"
        )
        || selected.implementation.is_some()
    {
        return Err("Byte position lacks exact pinned trait/implementation authority".into());
    }

    crate::data_library::verify_core_definition(
        &crate::data_library::callee(iterator)?,
        facts,
        crate::data_library::CoreDefinition {
            paths: &[
                "core::slice::<impl [T]>::iter",
                "std::slice::<impl [T]>::iter",
            ],
            unavailable: "Unpinned slice iterator build",
            mismatch: "Fresh slice iterator definition disagrees",
        },
    )?;
    signature(node, iterator, receiver, facts)
}

fn signature(
    node: &HirNode,
    iterator: &HirNode,
    receiver: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let observations = crate::data_receivers::verify(facts)?;
    let created = observations
        .methods
        .iter()
        .find(|record| record.call == iterator.hir)
        .ok_or("Missing slice iterator signature")?;
    let consumed = observations
        .methods
        .iter()
        .find(|record| record.call == node.hir)
        .ok_or("Missing position signature")?;

    fresh_signature(created, receiver, facts)?;
    consumed_signature(created, consumed, node)?;
    // Exact slice::iter authority and the shared-u8 instantiation fix the
    // internal fresh Iter<u8>; no general iterator value is admitted.
    dispatch(iterator)?;
    dispatch(node)
}

fn byte_slice() -> TypeTree {
    TypeTree::Slice(Box::new(TypeTree::Scalar("u8".into())))
}

fn fresh_signature(
    created: &crate::data_receivers::Method,
    receiver: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let implicit = crate::data_subslices::candidate(receiver, facts);
    if implicit
        && crate::data_subslices::verify(receiver, facts)?
            .array_length
            .is_some()
    {
        return Err("Fresh byte iterator requires an explicit shared array-field view".into());
    }
    let shared = TypeTree::Shared(Box::new(byte_slice()));
    let ty = if implicit {
        byte_slice()
    } else {
        shared.clone()
    };
    let slice = Observation::Captured { ty: shared };
    let adjustments: Vec<_> = created.adjustments.iter().map(|step| &step.kind).collect();
    let adjustment_ok = if implicit {
        adjustments == [&Kind::SharedBorrow]
    } else {
        adjustments.is_empty() || adjustments == [&Kind::BuiltinDereference, &Kind::SharedBorrow]
    };

    if created.receiver_type != (Observation::Captured { ty: ty.clone() })
        || created.adjusted_type != slice
        || created.signature_inputs != [slice]
        || !adjustment_ok
        || crate::data_aggregates::token(receiver)? != crate::data_types::token(&ty)?
    {
        return Err("Fresh byte iterator instantiation/receiver order disagrees".into());
    }
    Ok(())
}

fn consumed_signature(
    created: &crate::data_receivers::Method,
    consumed: &crate::data_receivers::Method,
    node: &HirNode,
) -> Result<(), String> {
    let result = Observation::Captured {
        ty: TypeTree::Option(Box::new(TypeTree::Scalar("usize".into()))),
    };
    if created.signature_result != consumed.receiver_type
        || !matches!(&created.signature_result, Observation::Unsupported { reason } if !reason.is_empty())
        || consumed.adjustments.len() != 1
        || consumed.adjustments[0].kind != Kind::MutableBorrow
        || consumed.adjustments[0].target != consumed.adjusted_type
        || consumed.signature_inputs.len() != 2
        || consumed.signature_result != result
        || consumed.signature_inputs[0] != consumed.adjusted_type
        || crate::data_aggregates::token(node)? != result_type()
    {
        return Err("Byte position loses its fresh cursor or concrete result".into());
    }
    Ok(())
}

fn dispatch(node: &HirNode) -> Result<(), String> {
    let callee = crate::data_library::callee(node)?;
    let ty = node
        .ty
        .as_ref()
        .ok_or("Missing iterator result observation")?;
    let results: Vec<_> = std::iter::once(ty.observed.as_str())
        .chain(ty.normalized.as_deref())
        .collect();
    crate::data_receivers::verify_dispatch(node, &callee.path, &results)
}
