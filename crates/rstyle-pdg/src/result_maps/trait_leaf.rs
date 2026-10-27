//! Exact expanded core TryFrom leaf within an independently checked whole map chain.
mod capture;
use crate::{
    HirFnFacts, HirNode,
    concrete_types::TypeTree,
    tryfrom_calls::{self, model},
    unit_store::EvidenceFacts,
};
use serde_json::{Value, json};
pub(crate) const CAPABILITY: &str = "rust.generated-checked-tryfrom-maps";
pub(crate) use capture::verify as capture;

/// Link candidate only; the owning unit later proves source, cleanup and callee effects.
pub(crate) fn link(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<Value> {
    if !candidate(node, facts) {
        return None;
    }
    let call = capture::selected(facts.facts(), node).ok()?;
    Some(resolution_row(&call))
}

fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && facts
            .build
            .frontend_options
            .contains_key("rstyle.rust.generated-conversion-member")
        && node.kind == "expr_call"
        && node.expansion.is_some()
        && node.dispatch.as_ref().is_some_and(|dispatch| {
            dispatch.op.replace("std::", "core::") == "core::convert::TryFrom::try_from"
        })
        && crate::result_maps::tail_candidate(facts)
}

pub(crate) fn resolution(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<Value> {
    candidate(node, facts)
        .then(|| checked_link(node, facts).ok())
        .flatten()
}

#[derive(serde::Serialize)]
struct ResolutionRow<'a> {
    local: bool,
    path: &'a str,
    definition_identity: &'a str,
    #[serde(rename = "crate")]
    crate_name: &'a str,
    generated_tryfrom_requested: &'a str,
}

fn resolution_row(call: &model::Call) -> Value {
    serde_json::to_value(ResolutionRow {
        local: true,
        path: &call.selected.path,
        definition_identity: &call.selected.definition_identity,
        crate_name: &call.selected.crate_name,
        generated_tryfrom_requested: &call.requested.path,
    })
    .expect("Generated conversion resolution has primitive fields")
}

fn checked_link(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<Value, String> {
    let resolution = facts.nominal_resolution()?;
    let source = resolution
        .transport_source()
        .ok_or("Missing generated conversion transport source")?;
    source.verify(facts.facts())?;
    let call = source
        .request()
        .trait_leaf()
        .ok_or("Missing exact generated trait leaf")?;
    if call.hir != node.hir {
        return Err("Generated trait link uses another HIR role".into());
    }
    let owner = source_owner(source.member().function())?;
    let TypeTree::Result { ok, err } = source.request().result_types()[0] else {
        unreachable!()
    };
    use crate::concrete_types::source_proof::PathResolution;
    if resolution.resolve(&owner)? != **ok
        || !resolution.checked_owned_copy(ok)
        || !err
            .nominal_identity()
            .is_some_and(|nominal| resolution.checked_no_drop(nominal))
    {
        return Err(
            "Generated trait leaf lacks exact lexical/Copy/no-drop payload ownership".into(),
        );
    }

    Ok(resolution_row(call))
}

pub(crate) fn verify_selection(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
    resolution: &Value,
) -> Result<(), String> {
    let checked = checked_link(node, caller)?;
    if &checked != resolution {
        return Err("Generated TryFrom resolution changed exact selected identity".into());
    }

    let nominal = caller.nominal_resolution()?;
    let source = nominal
        .transport_source()
        .ok_or("Missing generated conversion source")?;
    let call = source
        .request()
        .trait_leaf()
        .ok_or("Missing generated conversion leaf")?;
    let member = tryfrom_calls::verified_member(callee.facts())?;
    let (ok, err) = tryfrom_calls::result(callee.facts())?;
    if call.selected != member.method
        || call.requested != member.trait_method
        || call.requested_arguments != member.trait_arguments
        || call.selected_arguments != member.selected_arguments
        || source.request().result_types()[0]
            != &(TypeTree::Result {
                ok: Box::new(ok),
                err: Box::new(err),
            })
    {
        return Err(
            "Generated TryFrom leaf differs from original selected body and associated Error"
                .into(),
        );
    }
    Ok(())
}

pub(super) fn source_owner(function: &syn::ImplItemFn) -> Result<syn::TypePath, String> {
    let leaf = crate::inherent_calls::generated_leaf_source(function)?;
    let source = crate::inherent_calls::generated_call_owner(leaf)?;
    if source.method != "try_from" {
        return Err("Generated conversion leaf changes exact source method".into());
    }
    Ok(source.owner)
}
