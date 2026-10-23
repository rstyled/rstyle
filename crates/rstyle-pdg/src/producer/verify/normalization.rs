//! Canonical comparison preserves multiplicity and ordered fields. Keys for
//! unordered records are serialized once, rather than at every sort comparison.
use serde_json::Value;
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

pub(super) fn normalize(
    mut value: Value,
    field: &str,
    renaming: &BTreeMap<String, String>,
) -> Value {
    normalize_in_place(&mut value, field, renaming);
    value
}

fn normalize_in_place(value: &mut Value, field: &str, renaming: &BTreeMap<String, String>) {
    match value {
        Value::Object(object) => normalize_object(object, field, renaming),
        Value::Array(values) => normalize_array(values, field, renaming),
        _ => normalize_reference(value, field, renaming),
    }
}

fn normalize_object(
    object: &mut serde_json::Map<String, Value>,
    field: &str,
    renaming: &BTreeMap<String, String>,
) {
    object.retain(|key, value| {
        let keep = retained(field, key);
        if keep {
            normalize_in_place(value, key, renaming);
        }
        keep
    });
}

fn normalize_array(values: &mut [Value], field: &str, renaming: &BTreeMap<String, String>) {
    for value in values.iter_mut() {
        normalize_in_place(value, field, renaming);
    }
    if unordered(field) {
        values.sort_by_cached_key(Value::to_string);
    }
}

fn normalize_reference(value: &mut Value, field: &str, renaming: &BTreeMap<String, String>) {
    if let Value::String(text) = value
        && reference_field(field)
        && let Some(replacement) = renaming.get(text)
    {
        *text = replacement.clone();
    }
}

fn retained(field: &str, key: &str) -> bool {
    // Historical selection records carried an informational document path.
    // The registered profile identity remains authoritative.
    !(field == "conformance" && matches!(key, "status" | "reasons")
        || matches!(field, "edges" | "diagnostics") && key == "id"
        || field == crate::extraction_profile::EVIDENCE_KEY && key == "contract")
}

fn unordered(field: &str) -> bool {
    matches!(
        field,
        "nodes"
            | "edges"
            | "symbols"
            | "definitions"
            | "sources"
            | "diagnostics"
            | "alternatives"
            | "exits"
            | "reaching"
            | "memory_locations"
            | "entry_reachable_cfg_node_ids"
            | "completion_cfg_node_ids"
            | "compat.rust.hir-r0:operation-map"
            | "compat.rust.hir-r0:operation-provenance"
            | "compat.rust.hir-r0:value-sites"
            | "compat.rust.hir-r0:guards"
    )
}

fn reference_field(field: &str) -> bool {
    matches!(
        field,
        "id" | "location_id"
            | crate::data_places::KEY
            | "source"
            | "target"
            | "node_id"
            | "symbol_id"
            | "symbol_ids"
            | "defines"
            | "definition_id"
            | "source_id"
            | "cfg_node_id"
            | "cfg_operation_id"
            | "operation_id"
            | "pdg_node_id"
            | "controller_pdg_node_id"
            | "value_pdg_node_id"
            | "cfg_successor_node_id"
            | "completion_cfg_node_ids"
            | "entry_reachable_cfg_node_ids"
            | "entry_node_id"
            | "exit_node_ids"
            | "origin_node_ids"
    )
}
