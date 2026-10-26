//! Associate a concrete requested core trait member with its exact local body.
use super::{model, *};
use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use serde_json::Value;

pub(crate) struct Association {
    pub requested_path: String,
    pub selected_path: String,
    pub selected_identity: String,
    pub crate_name: String,
    pub owner_path: syn::Path,
    pub success: TypeTree,
    pub error: TypeTree,
}

pub(crate) struct SelectionInput<'a> {
    pub node: &'a HirNode,
    pub caller: &'a dyn EvidenceFacts,
    pub callee: &'a dyn EvidenceFacts,
    pub resolution: &'a Value,
}

pub(crate) fn association(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    authority: &dyn CoreAuthority,
) -> Result<Association, String> {
    let call = checked_call(node, caller, authority)?;
    let owner_path = original_source(node, caller)?;
    let (success, error) = super::result(caller)?;

    Ok(Association {
        requested_path: call.requested.path,
        selected_path: call.selected.path,
        selected_identity: call.selected.definition_identity,
        crate_name: call.selected.crate_name,
        owner_path,
        success,
        error,
    })
}

pub(crate) fn verify_selection(
    input: SelectionInput<'_>,
    authority: &dyn CoreAuthority,
) -> Result<(), String> {
    let call = checked_call(input.node, input.caller, authority)?;
    original_source(input.node, input.caller)?;
    let observed = super::observation(input.callee)?.ok_or("Missing selected TryFrom owner")?;
    owner::verify(input.callee, &observed, authority)?;
    if call.selected != observed.method
        || call.requested != observed.trait_method
        || call.requested_arguments != observed.trait_arguments
        || call.selected_arguments != observed.selected_arguments
        || super::result(input.caller)? != super::result(input.callee)?
    {
        return Err("TryFrom call substitutions differ from original selected body".into());
    }
    let resolution = input.resolution;
    if resolution["local"] != true
        || resolution["path"].as_str() != Some(call.selected.path.as_str())
        || resolution["definition_identity"].as_str()
            != Some(call.selected.definition_identity.as_str())
        || resolution["crate"].as_str() != Some(call.selected.crate_name.as_str())
    {
        return Err("TryFrom helper resolution differs from exact selected method".into());
    }
    // Same-build, lexical/source ownership, body effects and fresh-result exports
    // are independent mandatory checks in the surrounding helper pipeline.

    Ok(())
}

fn checked_call(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    authority: &dyn CoreAuthority,
) -> Result<model::Call, String> {
    if !crate::extraction_profile::facts_support(caller, CAPABILITY) {
        return Err("TryFrom caller lacks its registered capability".into());
    }
    let inventory = caller
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .ok_or("TryFrom caller lacks original call inventory")?;
    let call = original_call(inventory, node)?;
    owner::core(caller, CoreRole::Method, &call.requested, authority)?;
    owner::local(caller, &call.selected)?;
    let (success, _) = super::result(caller)?;
    let [self_argument, input] = call.requested_arguments.as_slice() else {
        return Err("TryFrom call requires concrete Self and exact unsigned substitutions".into());
    };
    let nominal = success
        .nominal_parts()
        .ok_or("TryFrom call success lacks nominal identity")?
        .0;
    let self_definition = owner::nominal(self_argument.known()?, nominal)?;
    owner::local(caller, &self_definition)?;
    input::structural(input.known()?, input::captured(caller.facts())?)?;
    if !call.selected_arguments.is_empty() || call.selected == call.requested {
        return Err("TryFrom call requires a distinct nongeneric local implementation".into());
    }
    requested(node, caller, &call)?;

    Ok(call)
}

pub(super) fn original_call(inventory: &Value, node: &HirNode) -> Result<model::Call, String> {
    if inventory["identity"] != "rstyle.rust.call-instance-observations/1"
        || inventory["complete"] != true
    {
        return Err("TryFrom requires complete original call inventory".into());
    }
    let calls = inventory["calls"]
        .as_array()
        .ok_or("Missing TryFrom call roster")?;
    let mut matching = calls.iter().filter(|call| {
        serde_json::from_value::<crate::HirRef>(call["hir"].clone())
            .ok()
            .as_ref()
            == Some(&node.hir)
            || serde_json::from_value::<crate::ByteSpan>(call["span"].clone()).ok()
                == Some(node.span)
    });
    let row = matching
        .next()
        .ok_or("Missing original TryFrom call site")?;
    if matching.next().is_some() {
        return Err("Ambiguous or duplicated original TryFrom call site".into());
    }
    let call: model::Call =
        serde_json::from_value(row.clone()).map_err(|error| error.to_string())?;
    if call.hir != node.hir
        || call.span != node.span
        || call.expanded
        || call.status != "resolved"
        || call.compiled_dependency.is_some()
    {
        return Err("TryFrom call is not an exact original local selection".into());
    }

    Ok(call)
}

fn requested(node: &HirNode, caller: &dyn EvidenceFacts, call: &model::Call) -> Result<(), String> {
    let resolution = node
        .res
        .as_ref()
        .filter(|resolution| resolution.kind == "resolved_callee" && resolution.target.is_none())
        .ok_or("TryFrom lacks original requested trait resolution")?;
    let observed: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    let registry =
        crate::data_library::registry_for(caller).ok_or("Unpinned TryFrom call compiler/target")?;
    if observed.path != call.requested.path
        || observed.crate_name != call.requested.crate_name
        || observed.definition_hash != call.requested.definition_identity
        || observed.crate_hash != registry.core_hash
        || observed.implementation.is_some()
    {
        return Err("TryFrom node contradicts actual requested core member".into());
    }
    let [operand] = node.children.as_slice() else {
        return Err("TryFrom requires its single evaluated exact unsigned argument".into());
    };
    let scalar = input::captured(caller.facts())?;
    if node.kind != "expr_call"
        || node.expansion.is_some()
        || operand.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(scalar)
        || node.dispatch.as_ref().is_none_or(|dispatch| {
            dispatch.kind != "resolved_direct"
                || dispatch.op != call.requested.path
                || dispatch.operand_tys != [scalar]
                || Some(dispatch.result_ty.as_str())
                    != node
                        .ty
                        .as_ref()
                        .map(|ty| ty.normalized.as_deref().unwrap_or(ty.observed.as_str()))
        })
    {
        return Err("TryFrom loses original scalar argument/result dispatch".into());
    }

    result_node(node, caller, &call.signature)
}

fn result_node(node: &HirNode, caller: &dyn EvidenceFacts, display: &Value) -> Result<(), String> {
    let ty = node
        .ty
        .as_ref()
        .ok_or("Missing TryFrom result observation")?;
    let (ok, err) = super::result(caller)?;
    let token = TypeToken::Result {
        ok: Box::new(TypeToken::from_tree(&ok)),
        err: Box::new(TypeToken::from_tree(&err)),
    }
    .text()?;
    if ty
        .normalized
        .as_ref()
        .is_some_and(|normalized| normalized != &token)
        || ty.observed != caller.signature.return_ty.observed
        || display["display_only"] != true
        || display["inputs"] != serde_json::json!([input::captured(caller.facts())?])
        || display["output"]
            .as_str()
            .map(|output| output.replace("std::", "core::"))
            != Some(ty.observed.replace("std::", "core::"))
    {
        return Err("TryFrom call result differs from complete captured caller Result".into());
    }

    Ok(())
}

fn original_source(node: &HirNode, caller: &dyn EvidenceFacts) -> Result<syn::Path, String> {
    let expression = crate::source_evidence::original_expression(node, caller)?;
    let syn::Expr::Call(call) = crate::data_aggregates::ungroup(&expression) else {
        return Err("TryFrom requires original direct-call source".into());
    };
    let syn::Expr::Path(path) = crate::data_aggregates::ungroup(&call.func) else {
        return Err("TryFrom requires an explicit original owner path".into());
    };
    if !call.attrs.is_empty()
        || !path.attrs.is_empty()
        || path.qself.is_some()
        || path.path.segments.len() < 2
        || path
            .path
            .segments
            .iter()
            .any(|part| !part.arguments.is_empty())
        || path
            .path
            .segments
            .last()
            .is_none_or(|part| part.ident != "try_from")
    {
        return Err("TryFrom original syntax changes owner/member/substitutions".into());
    }
    let [operand] = node.children.as_slice() else {
        return Err("Missing TryFrom operand".into());
    };
    let arguments: Vec<_> = call.args.iter().collect();
    let [argument] = arguments.as_slice() else {
        return Err("TryFrom source changes argument count".into());
    };
    crate::source_evidence::verify_child_source(argument, operand, node, caller)?;
    let mut owner = path.path.clone();
    owner.segments.pop();
    owner.segments.pop_punct();

    Ok(owner)
}
