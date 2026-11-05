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
pub fn bundle(mut documents: Vec<Value>) -> Result<Document, String> {
    let single = documents.len() == 1 && documents[0]["layout"].as_str() == Some("bundled");
    let bundled = match documents.pop() {
        Some(document) if single => document,
        last => {
            documents.extend(last);
            merge(documents)?
        }
    };

    let document: Document = serde_json::from_value(bundled)
        .map_err(|error| format!("Invalid compat.cpg document: {error}"))?;
    identity(&document)?;
    Ok(document)
}

/// One bundled document, read from the bytes its frontend wrote.
pub fn read(bytes: &[u8]) -> Result<Document, String> {
    let document: Document = serde_json::from_slice(bytes)
        .map_err(|error| format!("Invalid compat.cpg document: {error}"))?;
    identity(&document)?;
    Ok(document)
}

/// Every bundled unit document of a run: its bundled documents, and each split unit
/// document merged with the function documents that name it.
pub fn units(documents: Vec<Value>) -> Result<Vec<Document>, String> {
    let (bundled, split): (Vec<_>, Vec<_>) = documents
        .into_iter()
        .partition(|document| document["layout"].as_str() == Some("bundled"));
    let single = bundled.into_iter().map(|document| bundle(vec![document]));
    let merged = split_runs(split).into_iter().map(bundle);
    let units = single.chain(merged).collect::<Result<Vec<_>, _>>()?;
    distinct(&units)?;

    Ok(units)
}

/// A run may read one unit from several documents, as a frontend batches a large one,
/// but a function read twice would count twice.
fn distinct(units: &[Document]) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for unit in units.iter().filter_map(|document| document.unit.as_ref()) {
        for function in unit.functions.iter().flatten() {
            if !seen.insert((unit.id.as_str(), function.id.as_str())) {
                return Err(format!(
                    "Function {} of unit {} appears more than once in a run",
                    function.id, unit.id
                ));
            }
        }
    }
    Ok(())
}

/// Split documents by the unit document each belongs to, so that each set merges alone;
/// a function document naming none stays with the first set, which rejects it.
fn split_runs(documents: Vec<Value>) -> Vec<Vec<Value>> {
    let (units, functions): (Vec<_>, Vec<_>) = documents
        .into_iter()
        .partition(|document| document["document"].as_str() == Some("unit"));
    let references: Vec<Value> = units.iter().map(reference).collect();
    let mut runs: Vec<Vec<Value>> = units.into_iter().map(|unit| vec![unit]).collect();
    if runs.is_empty() && !functions.is_empty() {
        runs.push(Vec::new());
    }
    for function in functions {
        let owner = references
            .iter()
            .position(|reference| function["unit_ref"] == *reference)
            .unwrap_or(0);
        runs[owner].push(function);
    }

    runs
}

fn reference(unit: &Value) -> Value {
    serde_json::json!({ "id": unit["unit"]["id"], "digest": digest(unit) })
}

fn merge(documents: Vec<Value>) -> Result<Value, String> {
    if documents
        .iter()
        .any(|document| document["layout"].as_str() != Some("split"))
    {
        return Err("Every document of a run must be bundled or split".into());
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
    let unit_ref = reference(&unit);

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
