//! Reviewed inherent core definitions and Result implementation source.
use super::{model::Call, request::TransportRequest};
use crate::{HirFnFacts, HirNode, data_library::Callee};

pub(super) fn verify(
    facts: &HirFnFacts,
    request: &TransportRequest,
    nodes: [&HirNode; 2],
) -> Result<(), String> {
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Unpinned constructor Result transport compiler or target")?;
    crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "constructor Result maps")?;
    if registry.source_digests.get("result.rs").map(String::as_str)
        != Some("sha256:777e0f516deba3899c8f16c5511da3ebe7debe23f4bcf6a6353c98a927c0f7d1")
    {
        return Err("Constructor Result maps lack reviewed core Result source".into());
    }
    for ((node, call), name) in nodes
        .into_iter()
        .zip(request.calls())
        .zip(["map", "map_err"])
    {
        let callee = method(node, call, name)?;
        let key = format!("std::result::Result::<T, E>::{name}");
        if !registry.exact_core_definition(&callee, &key)
            || callee.definition_hash != call.selected.definition_identity
            || registry
                .definitions
                .get("rust.result-constructor-maps.core-crate")
                != Some(&call.selected.stable_crate_id)
        {
            return Err(
                "Constructor Result map lacks exact selected core definition authority".into(),
            );
        }
    }

    Ok(())
}

fn method(node: &HirNode, call: &Call, name: &str) -> Result<Callee, String> {
    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing constructor map method resolution")?;
    let callee: Callee =
        serde_json::from_str(&resolution.description).map_err(|e| e.to_string())?;
    dispatch(node, &callee)?;
    if resolution.kind != "resolved_callee"
        || resolution.target.is_some()
        || callee.path != format!("core::result::Result::<T, E>::{name}")
        || call.hir != node.hir
        || call.span != node.span
        || !call.expanded
        || node.binding.is_some()
        || node.lit.is_some()
        || node.is_unsafe_block
    {
        return Err("Constructor map resolution disagrees with its exact raw call site".into());
    }

    Ok(callee)
}

fn dispatch(node: &HirNode, callee: &Callee) -> Result<(), String> {
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing constructor map dispatch")?;
    let operands: Vec<_> = node
        .children
        .iter()
        .map(|child| child.ty.as_ref().map(|ty| ty.observed.as_str()))
        .collect();
    let expected: Vec<_> = dispatch
        .operand_tys
        .iter()
        .map(|ty| Some(ty.as_str()))
        .collect();
    if dispatch.kind != "resolved_method"
        || dispatch.op != callee.path
        || operands != expected
        || node.ty.as_ref().map(|ty| ty.observed.as_str()) != Some(dispatch.result_ty.as_str())
    {
        return Err("Constructor map dispatch changes its receiver, callback or result".into());
    }

    Ok(())
}
