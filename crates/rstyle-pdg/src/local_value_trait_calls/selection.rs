//! Keep requested local trait and selected original implementation distinct.
use super::{Contract, model, source};
use crate::{HirNode, unit_store::EvidenceFacts};
use serde_json::Value;

/// Observed association only. The source/trait checks and ordinary independently
/// verified callee body must succeed before using it as a helper dependency.
#[derive(serde::Serialize)]
pub(crate) struct Association {
    #[serde(rename = "value_trait_requested")]
    pub requested_path: String,
    #[serde(rename = "path")]
    pub selected_path: String,
    #[serde(rename = "definition_identity")]
    pub selected_identity: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
}

pub(crate) fn association(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
) -> Result<Association, String> {
    let call = checked_call(node, caller)?;

    Ok(Association {
        requested_path: call.requested.path,
        selected_path: call.selected.path,
        selected_identity: call.selected.definition_identity,
        crate_name: call.selected.crate_name,
    })
}

pub(crate) fn verify(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
    resolution: &Value,
) -> Result<(), String> {
    let call = checked_call(node, caller)?;
    call.selected.selected(callee)?;
    if resolution["local"] != true
        || resolution["path"].as_str() != Some(call.selected.path.as_str())
        || resolution["definition_identity"].as_str()
            != Some(call.selected.definition_identity.as_str())
        || resolution["crate"].as_str() != Some(call.selected.crate_name.as_str())
    {
        return Err(
            "Local value trait helper link differs from exact selected original body".into(),
        );
    }

    let contract = call_contract(&call)?;
    if source::captured_signature(callee)? != contract
        || !crate::extraction_profile::facts_support(callee, contract.capability())
    {
        return Err(
            "Local value trait callee changes the independently gated primitive route".into(),
        );
    }
    // Ordinary same-build, source-namespace and callee body/effects verification
    // remains mandatory in the surrounding helper pipeline.
    Ok(())
}

fn checked_call(node: &HirNode, caller: &dyn EvidenceFacts) -> Result<model::Call, String> {
    let inventory = caller
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .ok_or("Local value trait lacks its independent original call inventory")?;
    let call = original_call(inventory, node)?;
    let contract = call_contract(&call)?;
    if !crate::extraction_profile::facts_support(caller, contract.capability()) {
        return Err("Local value trait call requires its registered route capability".into());
    }
    call.requested.local(caller)?;
    call.selected.local(caller)?;

    relationship(&call)?;
    requested(node, &call)?;
    original_source(node, caller, &call)?;
    Ok(call)
}

pub(super) fn original_call(inventory: &Value, node: &HirNode) -> Result<model::Call, String> {
    if inventory["identity"] != "rstyle.rust.call-instance-observations/1"
        || inventory["complete"] != true
    {
        return Err("Local value trait requires a complete original call inventory".into());
    }
    let calls = inventory["calls"]
        .as_array()
        .ok_or("Local value trait call roster missing")?;
    let mut matching = calls.iter().filter(|call| {
        serde_json::from_value::<crate::HirRef>(call["hir"].clone())
            .ok()
            .as_ref()
            == Some(&node.hir)
            || serde_json::from_value::<crate::ByteSpan>(call["span"].clone()).ok()
                == Some(node.span)
    });
    let selected = matching
        .next()
        .ok_or("Local value trait original call site missing")?;
    if matching.next().is_some() {
        return Err("Local value trait original call site is duplicated or ambiguous".into());
    }

    let call: model::Call =
        serde_json::from_value(selected.clone()).map_err(|error| error.to_string())?;
    if call.hir != node.hir
        || call.span != node.span
        || call.status != "resolved"
        || call.expanded
        || call.compiled_dependency.is_some()
        || call.signature["display_only"] != true
        || call_contract(&call).is_err()
    {
        return Err("Local value trait call lacks exact original resolved scalar signature".into());
    }
    Ok(call)
}

pub(super) fn relationship(call: &model::Call) -> Result<(), String> {
    let contract = call_contract(call)?;
    model::self_argument(&call.requested_arguments, contract.input())?;
    let (trait_path, member) = source::selected_path(&call.selected.path, contract)?;
    let expected = trait_path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .chain(std::iter::once(member))
        .collect::<Vec<_>>()
        .join("::");

    if call.requested.path != expected
        || !call.selected_arguments.is_empty()
        || !call.requested.local
        || !call.selected.local
        || call.requested.definition_identity.is_empty()
        || call.selected.definition_identity.is_empty()
        || call.requested.definition_identity == call.selected.definition_identity
        || call.requested.stable_crate_id.is_empty()
        || call.requested.stable_crate_id != call.selected.stable_crate_id
        || call.requested.crate_name.is_empty()
        || call.requested.crate_name != call.selected.crate_name
    {
        return Err(
            "Local value trait requested member and selected implementation disagree".into(),
        );
    }
    Ok(())
}

fn requested(node: &HirNode, call: &model::Call) -> Result<(), String> {
    let contract = call_contract(call)?;
    let resolution = node
        .res
        .as_ref()
        .filter(|resolution| resolution.kind == "resolved_callee" && resolution.target.is_none())
        .ok_or("Local value trait lacks original requested method resolution")?;
    let observed: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    if observed.path != call.requested.path
        || observed.crate_name != call.requested.crate_name
        || observed.definition_hash != call.requested.definition_identity
        || observed.crate_hash != "local-unencoded"
        || observed.implementation.is_some()
    {
        return Err("Local value trait node disagrees with original requested trait member".into());
    }

    let [operand] = node.children.as_slice() else {
        return Err("Local value trait call requires one value receiver".into());
    };
    if node.kind != "expr_method_call"
        || node.expansion.is_some()
        || node.dispatch.as_ref().is_none_or(|dispatch| {
            dispatch.kind != "resolved_method"
                || dispatch.op != call.requested.path
                || dispatch.operand_tys != [contract.input()]
                || dispatch.result_ty != contract.output()
        })
        || node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(contract.output())
        || operand.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(contract.input())
    {
        return Err(
            "Local value trait loses its original primitive receiver/result dispatch".into(),
        );
    }
    Ok(())
}

fn original_source(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    call: &model::Call,
) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, caller)?;
    let syn::Expr::MethodCall(method) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Local value trait requires original method-call syntax".into());
    };
    let (_, name) = source::selected_path(&call.selected.path, call_contract(call)?)?;
    if !method.args.is_empty()
        || method.turbofish.is_some()
        || !method.attrs.is_empty()
        || method.method != name
    {
        return Err("Local value trait original invocation changes member or arguments".into());
    }

    let [operand] = node.children.as_slice() else {
        return Err("Local value trait original receiver missing".into());
    };
    crate::source_evidence::verify_child_source(method.receiver.as_ref(), operand, node, caller)
}

fn call_contract(call: &model::Call) -> Result<Contract, String> {
    let inputs = call.signature["inputs"]
        .as_array()
        .ok_or("Local value trait call signature lacks concrete inputs")?;
    let [input] = inputs.as_slice() else {
        return Err("Local value trait call signature requires one primitive input".into());
    };
    input
        .as_str()
        .zip(call.signature["output"].as_str())
        .and_then(|(input, output)| Contract::from_types(input, output))
        .ok_or("Local value trait call signature is outside its exact primitive routes".into())
}

#[cfg(test)]
mod dispatch_tests;
