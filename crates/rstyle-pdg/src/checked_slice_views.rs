//! Checked prefix views retain saved receiver/end metadata, never content reads.
mod authority;
mod source;
#[cfg(test)]
mod tests;
mod types;
use crate::{HirNode, call_summaries::Summary, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.checked-borrowed-result-flow";
pub(crate) const KEY: &str = "rstyle.data:checked-prefix-view";
pub(crate) const CONSTRUCT: &str = "data.option-get-prefix";
const GET: &str = "core::slice::<impl [T]>::get";

pub(crate) struct Checked<'a> {
    pub receiver: &'a HirNode,
    pub range: &'a HirNode,
    pub end: &'a HirNode,
}

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && node.kind == "expr_method_call"
        && node
            .dispatch
            .as_ref()
            .is_some_and(|dispatch| dispatch.op == GET)
        && node
            .children
            .get(1)
            .is_some_and(|range| range.kind == "expr_struct")
}

pub(crate) fn verify<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Checked<'a>, String> {
    if !candidate(node, facts) || node.expansion.is_some() {
        return Err("Checked prefix get requires its original registered method".into());
    }
    let [receiver, range] = node.children.as_slice() else {
        return Err("Checked prefix get requires exactly receiver and range".into());
    };
    let [end] = range.children.as_slice() else {
        return Err("Checked prefix get requires exactly one range endpoint".into());
    };
    let checked = Checked {
        receiver,
        range,
        end,
    };
    types::verify(node, &checked, facts)?;
    authority::verify(node, &checked, facts)?;
    source::verify(node, &checked, facts)?;

    Ok(checked)
}

pub(crate) fn sources(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::field_locations::selected_operations(facts, &["expr_method_call"], candidate)
}

pub(crate) fn resolve(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<Summary, String> {
    verify(node, facts)?;
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing prefix get dispatch")?;

    Ok(Summary {
        id: "rstyle.data.slice.get-prefix".into(),
        version: "1".into(),
        callee_path: GET.into(),
        callee_crate: "core".into(),
        argument_types: dispatch.operand_tys.clone(),
        result_type: dispatch.result_ty.clone(),
        memory_effects: false,
        mutates_arguments: false,
        may_panic: false,
        always_returns: true,
        overflow_behavior: "not_applicable".into(),
    })
}

#[derive(serde::Serialize)]
struct Semantics {
    rule: &'static str,
    receiver_operand: u64,
    end_operand: u64,
    some_iff: &'static str,
    start: u64,
    some_length: &'static str,
    some_origins: &'static str,
    none: &'static str,
    content_read: bool,
    may_panic: bool,
    evaluation_order: [&'static str; 2],
}

pub(crate) fn semantics() -> serde_json::Value {
    let semantics = Semantics {
        rule: "checked-saved-prefix-view/1",
        receiver_operand: 0,
        end_operand: 1,
        some_iff: "end <= saved_receiver_length",
        start: 0,
        some_length: "saved_end",
        some_origins: "saved_receiver",
        none: "end > saved_receiver_length",
        content_read: false,
        may_panic: false,
        evaluation_order: ["receiver", "end"],
    };

    serde_json::to_value(semantics).expect("registered checked view semantics")
}
