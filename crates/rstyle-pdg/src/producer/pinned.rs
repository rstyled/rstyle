//! Checked bridge between the pinned record-policy PDG wire contract and the
//! workspace root-profile transport. Policies must agree before conversion;
//! no imported matching semantics or validator requirements are relaxed.

use crate::{PdgDocument, ProfileId, R0_POLICY_ID, R0_POLICY_VERSION};
use serde_json::{Value, json};

pub fn pdg_value(document: &PdgDocument) -> Result<Value, String> {
    let mut value = serde_json::to_value(document).map_err(|e| e.to_string())?;
    transform_records(&mut value, true)?;
    let conformance = value
        .pointer_mut("/extraction_evidence/conformance")
        .ok_or("Missing conformance")?;
    let mut record = conformance.take();
    record
        .as_object_mut()
        .ok_or("Malformed conformance")?
        .insert("profile".to_string(), profile());
    *conformance = Value::Array(vec![record]);

    Ok(value)
}

pub fn read_pdg(bytes: &[u8]) -> Result<PdgDocument, String> {
    let mut value: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if value.get("profile") != Some(&profile()) {
        return Err("Pinned R0 root profile mismatch".to_string());
    }
    let encoded = value
        .pointer("/extraction_evidence/conformance")
        .ok_or("Missing conformance")?;
    let decoded = read_conformance(encoded)?;
    *value
        .pointer_mut("/extraction_evidence/conformance")
        .expect("conformance field") = decoded;
    transform_records(&mut value, false)?;

    serde_json::from_value(value).map_err(|e| e.to_string())
}

fn read_conformance(value: &Value) -> Result<Value, String> {
    let records = value
        .as_array()
        .ok_or("Pinned conformance must be an array")?;
    let [record] = records.as_slice() else {
        return Err("Expected exactly one R0 conformance assessment".to_string());
    };
    let mut record = record.clone();
    remove_policy(&mut record, "profile", true)?;

    Ok(record)
}

fn profile() -> Value {
    json!(ProfileId {
        id: R0_POLICY_ID.to_string(),
        version: R0_POLICY_VERSION.to_string()
    })
}

fn remove_policy(record: &mut Value, field: &str, required: bool) -> Result<(), String> {
    let object = record
        .as_object_mut()
        .ok_or("Policy record must be an object")?;
    let value = object.remove(field);
    if (required && value.is_none()) || value.is_some_and(|value| value != profile()) {
        return Err(format!("Missing or mismatched pinned {field}"));
    }

    Ok(())
}

fn policy(record: &mut Value, writing: bool, required: bool) -> Result<(), String> {
    if writing {
        record
            .as_object_mut()
            .ok_or("Policy record must be an object")?
            .insert("policy".to_string(), profile());
        Ok(())
    } else {
        remove_policy(record, "policy", required)
    }
}

fn transform_records(document: &mut Value, writing: bool) -> Result<(), String> {
    transform_operations(document, writing)?;
    transform_capabilities(document, writing)?;
    transform_metadata(document, writing)
}

fn transform_operations(document: &mut Value, writing: bool) -> Result<(), String> {
    for field in ["nodes", "edges"] {
        for record in document
            .get_mut(field)
            .and_then(Value::as_array_mut)
            .ok_or("Missing node/edge records")?
        {
            if field == "edges" {
                policy(record, writing, true)?;
            }
            transform_evidence(record, writing)?;
        }
    }
    Ok(())
}

fn transform_metadata(document: &mut Value, writing: bool) -> Result<(), String> {
    policy(
        document
            .pointer_mut("/function/interface_completeness")
            .ok_or("Missing interface evidence")?,
        writing,
        false,
    )?;
    let count = document
        .pointer_mut("/function/reference_count")
        .ok_or("Missing reference count")?;
    if count.get("status").and_then(Value::as_str) == Some("known") {
        policy(count, writing, true)?;
    }

    Ok(())
}

fn transform_evidence(record: &mut Value, writing: bool) -> Result<(), String> {
    for field in ["evidence", "provenance"] {
        policy(
            record
                .get_mut(field)
                .ok_or("Missing node/edge evidence/provenance")?,
            writing,
            false,
        )?;
    }

    Ok(())
}

fn transform_capabilities(document: &mut Value, writing: bool) -> Result<(), String> {
    transform_capability_map(document, writing)?;
    transform_locations(document, writing)
}

fn transform_capability_map(document: &mut Value, writing: bool) -> Result<(), String> {
    let capabilities = document
        .pointer_mut("/extraction_evidence/capabilities")
        .and_then(Value::as_object_mut)
        .ok_or("Missing capabilities")?;
    capabilities
        .values_mut()
        .try_for_each(|value| policy(value, writing, false))
}

fn transform_locations(document: &mut Value, writing: bool) -> Result<(), String> {
    let locations = document
        .get_mut("memory_locations")
        .and_then(Value::as_array_mut)
        .ok_or("Missing memory locations")?;

    locations
        .iter_mut()
        .try_for_each(|location| transform_location(location, writing))
}

fn transform_location(location: &mut Value, writing: bool) -> Result<(), String> {
    policy(
        location
            .get_mut("evidence")
            .ok_or("Missing memory evidence")?,
        writing,
        false,
    )
}
