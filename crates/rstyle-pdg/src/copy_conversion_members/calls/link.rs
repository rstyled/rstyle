//! Raw local call candidates only. A link never proves callable authority.
use super::*;

#[cfg(test)]
mod tests;

pub(crate) fn selected_resolution(node: &HirNode, caller: &dyn EvidenceFacts) -> Option<Value> {
    let call = original_call(node, caller).ok()?;
    candidate(&call, node, caller).ok()?;
    serde_json::to_value(Link {
        local: true,
        path: &call.selected.path,
        definition_identity: &call.selected.definition_identity,
        crate_name: &call.selected.crate_name,
    })
    .ok()
}

fn candidate(call: &model::Call, node: &HirNode, caller: &dyn EvidenceFacts) -> Result<(), String> {
    let role = match call.requested.path.replace("std::", "core::").as_str() {
        "core::convert::From::from" => "from-method",
        "core::convert::TryFrom::try_from" => "tryfrom-method",
        _ => return Err("Not a tagged conversion requested core member".into()),
    };
    authority::reviewed(caller)?.role(&call.requested, role)?;
    let owner = super::super::verify::Owner::from_facts(caller.facts())?;
    if !call.selected.local
        || call.selected.path.is_empty()
        || call.selected.definition_identity.is_empty()
        || call.selected.crate_name != owner.crate_name
        || call.selected.stable_crate_id != owner.stable_crate_id
        || !call.selected_arguments.is_empty()
        || call.requested_arguments.len() != 2
        || call.compiled_dependency.is_some()
    {
        return Err(
            "Tagged conversion candidate must select an original same-unit concrete member".into(),
        );
    }

    for argument in &call.requested_arguments {
        argument.known()?;
    }
    let [actual] = node.children.as_slice() else {
        return Err("Tagged conversion candidate requires one whole-value actual".into());
    };
    candidate_types(actual, node, role)
}

fn candidate_types(actual: &HirNode, node: &HirNode, role: &str) -> Result<(), String> {
    use crate::data_types::TypeToken;
    let input = crate::data_aggregates::parsed_type(actual)?;
    let output = crate::data_aggregates::parsed_type(node)?;

    let supported = match (role, &input, &output) {
        ("from-method", TypeToken::Struct { .. }, TypeToken::MixedEnum { .. }) => true,
        ("tryfrom-method", TypeToken::MixedEnum { .. }, TypeToken::Result { ok, err }) => {
            matches!((ok.as_ref(), err.as_ref()), (TypeToken::Struct { .. }, TypeToken::Struct { fields, .. }) if fields.is_empty())
        }
        _ => false,
    };
    if !supported {
        return Err(
            "Tagged conversion link requires a complete product/tagged/Result shape".into(),
        );
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct Link<'a> {
    local: bool,
    path: &'a str,
    definition_identity: &'a str,
    #[serde(rename = "crate")]
    crate_name: &'a str,
}
