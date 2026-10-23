//! Checked suffix views retain saved receiver/start metadata, never content reads.
mod authority;
mod source;
#[cfg(test)]
mod tests;
mod types;
use crate::{HirNode, call_summaries::Summary, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.checked-suffix-result-views";
pub(crate) const KEY: &str = "rstyle.data:checked-suffix-view";
pub(crate) const CONSTRUCT: &str = "data.option-get-suffix";
const GET: &str = "core::slice::<impl [T]>::get";

pub(crate) struct Checked<'a> {
    pub receiver: &'a HirNode,
    pub range: &'a HirNode,
    pub start: &'a HirNode,
}

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    super::shared::candidate(node, facts, super::shared::RangeKind::Suffix)
}

pub(crate) fn verify<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Checked<'a>, String> {
    if !candidate(node, facts) || node.expansion.is_some() {
        return Err("Checked suffix get requires its original registered method".into());
    }
    let [receiver, range] = node.children.as_slice() else {
        return Err("Checked suffix get requires exactly receiver and range".into());
    };
    let [start] = range.children.as_slice() else {
        return Err("Checked suffix get requires exactly one range start".into());
    };
    let checked = Checked {
        receiver,
        range,
        start,
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
        .ok_or("Missing suffix get dispatch")?;

    Ok(Summary {
        id: "rstyle.data.slice.get-suffix".into(),
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
    start_operand: u64,
    some_iff: &'static str,
    some_start: &'static str,
    some_length: &'static str,
    some_pointer: &'static str,
    some_origins: &'static str,
    empty_pointer: &'static str,
    none: &'static str,
    content_read: bool,
    may_panic: bool,
    evaluation_order: [&'static str; 2],
    evaluation_count: &'static str,
}

pub(crate) fn semantics() -> serde_json::Value {
    let semantics = Semantics {
        rule: "checked-saved-suffix-view/1",
        receiver_operand: 0,
        start_operand: 1,
        some_iff: "saved_start <= saved_receiver_length",
        some_start: "saved_start",
        some_length: "saved_receiver_length - saved_start",
        some_pointer: "saved_receiver_pointer + saved_start",
        some_origins: "saved_receiver",
        empty_pointer: "saved_receiver_pointer + saved_receiver_length",
        none: "saved_start > saved_receiver_length",
        content_read: false,
        may_panic: false,
        evaluation_order: ["receiver", "start"],
        evaluation_count: "each-once",
    };

    serde_json::to_value(semantics).expect("registered checked suffix view semantics")
}
