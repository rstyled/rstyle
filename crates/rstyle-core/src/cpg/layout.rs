//! Loads a run in either layout. A split run is merged into the bundled document it
//! was split from; a function document is paired with its unit only by the unit
//! document's identity and canonical digest.

use super::model::{Document, DocumentKind, Layout};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const PROFILE: &str = "compat.cpg";
pub const VERSION: &str = "0.1.0";
const SHARED: [&str; 4] = ["schema_version", "profile", "producer", "coverage"];

/// Canonical JSON: object keys sorted by code point, no insignificant whitespace.
pub fn canonical(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            let members: Vec<String> = entries
                .into_iter()
                .map(|(key, item)| format!("{}:{}", Value::String(key.clone()), canonical(item)))
                .collect();
            format!("{{{}}}", members.join(","))
        }
        Value::Array(items) => {
            let items: Vec<String> = items.iter().map(canonical).collect();
            format!("[{}]", items.join(","))
        }
        scalar => scalar.to_string(),
    }
}

/// `sha256:` and the lowercase hex digest of the canonical JSON.
pub fn digest(value: &Value) -> String {
    let hash = Sha256::digest(canonical(value).as_bytes());
    let hex: String = hash.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("sha256:{hex}")
}

/// The bundled unit document of a run: one bundled document, or a split set.
pub fn bundle(documents: Vec<Value>) -> Result<Document, String> {
    let bundled = match documents.as_slice() {
        [single] if single["layout"].as_str() == Some("bundled") => single.clone(),
        _ => merge(documents)?,
    };
    let document: Document = serde_json::from_value(bundled)
        .map_err(|error| format!("Invalid compat.cpg document: {error}"))?;
    identity(&document)?;
    Ok(document)
}

fn merge(documents: Vec<Value>) -> Result<Value, String> {
    if documents
        .iter()
        .any(|document| document["layout"].as_str() != Some("split"))
    {
        return Err("A run is one bundled document or a set of split documents".into());
    }
    let (units, functions): (Vec<_>, Vec<_>) = documents
        .into_iter()
        .partition(|document| document["document"].as_str() == Some("unit"));
    let [unit] = <[Value; 1]>::try_from(units).map_err(|units| {
        format!(
            "A split run needs exactly one unit document, found {}",
            units.len()
        )
    })?;
    let unit_ref = serde_json::json!({ "id": unit["unit"]["id"], "digest": digest(&unit) });

    let bodies = functions
        .into_iter()
        .map(|function| paired(function, &unit, &unit_ref))
        .collect::<Result<Vec<_>, _>>()?;
    with_bodies(unit, bodies)
}

fn with_bodies(mut unit: Value, bodies: Vec<Value>) -> Result<Value, String> {
    unit["layout"] = Value::from("bundled");
    let tables = unit["unit"]
        .as_object_mut()
        .ok_or("The unit document has no unit")?;
    tables.insert("functions".into(), Value::Array(bodies));
    Ok(unit)
}

fn paired(mut function: Value, unit: &Value, unit_ref: &Value) -> Result<Value, String> {
    let name = function["function"]["id"]
        .as_str()
        .unwrap_or("<unnamed>")
        .to_string();

    if function["unit_ref"] != *unit_ref {
        return Err(format!("Function {name} belongs to another unit document"));
    }
    if let Some(differs) = SHARED.iter().find(|key| function[**key] != unit[**key]) {
        return Err(format!(
            "Function {name} differs from its unit in {differs}"
        ));
    }
    let body = function
        .as_object_mut()
        .and_then(|map| map.remove("function"));
    body.ok_or_else(|| format!("Function document {name} has no function"))
}

fn identity(document: &Document) -> Result<(), String> {
    let profile = &document.profile;
    if document.schema_version != VERSION || profile.id != PROFILE || profile.version != VERSION {
        return Err(format!(
            "Unsupported document: schema {} of profile {}/{}; this rstyle reads {PROFILE}/{VERSION}",
            document.schema_version, profile.id, profile.version
        ));
    }
    if document.document != DocumentKind::Unit || document.layout != Layout::Bundled {
        return Err("A run must merge into one bundled unit document".into());
    }
    match &document.unit {
        Some(unit) if unit.functions.is_some() => Ok(()),
        _ => Err("The bundled unit document has no function bodies".into()),
    }
}

/// Inserts `layout` and `unit_ref` so a test can split a bundled run.
#[cfg(test)]
pub fn split(bundled: &Value) -> Vec<Value> {
    let mut unit = bundled.clone();
    unit["layout"] = Value::from("split");
    let functions = unit["unit"]
        .as_object_mut()
        .unwrap()
        .remove("functions")
        .unwrap();
    let unit_ref = serde_json::json!({ "id": unit["unit"]["id"], "digest": digest(&unit) });
    let mut documents = vec![unit.clone()];
    for function in functions.as_array().unwrap() {
        let mut document = serde_json::Map::new();
        for key in SHARED {
            document.insert(key.into(), unit[key].clone());
        }
        document.insert("layout".into(), Value::from("split"));
        document.insert("document".into(), Value::from("function"));
        document.insert("unit_ref".into(), unit_ref.clone());
        document.insert("function".into(), function.clone());
        documents.push(Value::Object(document));
    }
    documents
}
