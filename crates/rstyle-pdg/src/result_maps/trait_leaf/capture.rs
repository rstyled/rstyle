use super::*;
use crate::generated_sources::checked::CheckedMember;

pub(crate) fn verify(
    facts: &HirFnFacts,
    member: &CheckedMember,
    result: &TypeTree,
    node: &HirNode,
) -> Result<Option<model::Call>, String> {
    if member.implementation().trait_.is_none() {
        return Ok(None);
    }
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Generated TryFrom maps require their registered capability".into());
    }

    source_owner(member.function())?;
    let call = selected(facts, node)?;
    types(facts, &call, result)?;
    requested(facts, &call, node, result)?;
    Ok(Some(call))
}

pub(super) fn selected(facts: &HirFnFacts, node: &HirNode) -> Result<model::Call, String> {
    let raw = facts
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .ok_or("Missing generated conversion call roster")?;
    if raw["identity"] != "rstyle.rust.call-instance-observations/1" || raw["complete"] != true {
        return Err("Incomplete generated conversion call inventory".into());
    }

    let calls = raw["calls"]
        .as_array()
        .ok_or("Missing generated conversion call rows")?;
    let mut selected = calls.iter().filter(|call| {
        serde_json::from_value::<crate::HirRef>(call["hir"].clone())
            .ok()
            .as_ref()
            == Some(&node.hir)
    });
    let row = selected
        .next()
        .ok_or("Missing generated TryFrom leaf call")?;
    if selected.next().is_some() {
        return Err("Duplicated generated TryFrom leaf call".into());
    }

    let call: model::Call = serde_json::from_value(row.clone()).map_err(|e| e.to_string())?;
    if call.hir != node.hir
        || call.span != node.span
        || !call.expanded
        || call.status != "resolved"
        || call.compiled_dependency.is_some()
        || !call.selected_arguments.is_empty()
        || call.selected == call.requested
    {
        return Err("Generated TryFrom leaf lacks an exact local selected call".into());
    }
    tryfrom_calls::verify_core(
        facts,
        tryfrom_calls::CoreRole::Method,
        &call.requested,
        &tryfrom_calls::RegistryAuthority,
    )?;
    tryfrom_calls::verify_local(facts, &call.selected)?;
    Ok(call)
}

fn types(facts: &HirFnFacts, call: &model::Call, result: &TypeTree) -> Result<(), String> {
    let scalar = tryfrom_calls::input::captured(facts)?;
    let TypeTree::Result { ok, .. } = result else {
        unreachable!()
    };
    let [owner, input] = call.requested_arguments.as_slice() else {
        return Err("Generated TryFrom lacks exact Self/input substitutions".into());
    };

    let nominal = ok
        .nominal_identity()
        .ok_or("Missing generated leaf nominal result")?;
    let definition = tryfrom_calls::verify_nominal(owner.known()?, nominal)?;
    tryfrom_calls::verify_local(facts, &definition)?;
    if !matches!(input.known()?, model::Structural::Builtin {name} if name == scalar)
        || call.selected.path.replace("std::", "core::")
            != format!(
                "<{} as core::convert::TryFrom<{scalar}>>::try_from",
                nominal.definition_path
            )
    {
        return Err("Generated TryFrom substitutes another owner/input/selected method".into());
    }
    Ok(())
}

fn requested(
    facts: &HirFnFacts,
    call: &model::Call,
    node: &HirNode,
    result: &TypeTree,
) -> Result<(), String> {
    let resolution = node
        .res
        .as_ref()
        .filter(|res| res.kind == "resolved_callee" && res.target.is_none())
        .ok_or("Missing generated requested core resolution")?;
    let observed: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|e| e.to_string())?;
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Unpinned generated TryFrom compiler/target")?;
    if observed.path != call.requested.path
        || observed.crate_name != call.requested.crate_name
        || observed.definition_hash != call.requested.definition_identity
        || observed.crate_hash != registry.core_hash
        || observed.implementation.is_some()
    {
        return Err("Generated leaf node contradicts exact requested core member".into());
    }

    dispatch(facts, call, node, result)
}

fn dispatch(
    facts: &HirFnFacts,
    call: &model::Call,
    node: &HirNode,
    result: &TypeTree,
) -> Result<(), String> {
    let scalar = tryfrom_calls::input::captured(facts)?;
    let token = crate::data_types::token(result)?;
    let [operand] = node.children.as_slice() else {
        return Err("Generated TryFrom lacks its single input".into());
    };
    let ty = node
        .ty
        .as_ref()
        .ok_or("Missing generated leaf result type")?;

    if node.kind != "expr_call"
        || node.expansion.is_none()
        || operand.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(scalar)
        || ty.normalized.as_deref() != Some(token.as_str())
        || node.dispatch.as_ref().is_none_or(|dispatch| {
            dispatch.kind != "resolved_direct"
                || dispatch.op != call.requested.path
                || dispatch.operand_tys != [scalar]
                || dispatch.result_ty != token
        })
        || call.signature["display_only"] != true
        || call.signature["inputs"] != json!([scalar])
        || call.signature["output"]
            .as_str()
            .map(|output| output.replace("std::", "core::"))
            != Some(ty.observed.replace("std::", "core::"))
    {
        return Err("Generated TryFrom loses exact input/result dispatch evidence".into());
    }
    Ok(())
}
