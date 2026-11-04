//! Diagnostic: the numeric style inputs read from the documents, compared with the
//! compiler capture they replace. `RSTYLE_DIAGNOSTIC_STYLE_DIR` receives one JSON-lines
//! file per crate naming each function whose inputs differ.

use rstyle_pdg::StyleFnFacts;
use rustc_middle::ty::TyCtxt;
use serde_json::{Value, json};
use std::io::Write;

pub const DIR_ENV: &str = "RSTYLE_DIAGNOSTIC_STYLE_DIR";

// rstyle-allow(side_effect_density): diagnostic boundary: writes the comparison it was asked for
/// Writes each function's differing fields, when a comparison is requested.
pub(super) fn compare(
    tcx: TyCtxt<'_>,
    derived: &[StyleFnFacts],
    reference: impl FnOnce() -> Result<Vec<StyleFnFacts>, String>,
) -> Result<(), String> {
    let Some(dir) = std::env::var_os(DIR_ENV) else {
        return Ok(());
    };
    let lines: Vec<Value> = match reference() {
        Err(error) => vec![json!({"function": null, "reference_error": error})],
        Ok(reference) if reference.len() != derived.len() => {
            vec![json!({"function": null, "reference_error":
                format!("{} reference functions, {} read", reference.len(), derived.len())})]
        }
        Ok(reference) => reference
            .iter()
            .zip(derived)
            .map(|(reference, derived)| {
                json!({"function": reference.name, "differences": differences(reference, derived)})
            })
            .collect(),
    };
    let name = format!(
        "{}-{:016x}-{}.jsonl",
        tcx.crate_name(rustc_span::def_id::LOCAL_CRATE),
        tcx.stable_crate_id(rustc_span::def_id::LOCAL_CRATE)
            .as_u64(),
        std::process::id()
    );
    let path = std::path::Path::new(&dir).join(name);
    let mut file =
        std::fs::File::create(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    for line in lines {
        writeln!(file, "{line}").map_err(|error| error.to_string())?;
    }

    Ok(())
}

/// The top-level fields, and frontend options, whose values differ.
fn differences(reference: &StyleFnFacts, derived: &StyleFnFacts) -> Vec<Value> {
    let (reference, derived) = (fields(reference), fields(derived));
    let mut keys: Vec<&String> = reference.keys().chain(derived.keys()).collect();
    keys.sort();
    keys.dedup();

    keys.into_iter()
        .filter(|key| reference.get(*key) != derived.get(*key))
        .map(
            |key| json!({"key": key, "reference": reference.get(key), "derived": derived.get(key)}),
        )
        .collect()
}

/// A function's fields, its frontend options among them; the event lists of its
/// advisory observations as multisets, as the core's equivalence compares them.
fn fields(facts: &StyleFnFacts) -> serde_json::Map<String, Value> {
    let Ok(Value::Object(mut map)) = serde_json::to_value(facts) else {
        return serde_json::Map::new();
    };
    let options = map
        .get_mut("build")
        .and_then(|build| build.as_object_mut())
        .and_then(|build| build.remove("frontend_options"));
    let Some(Value::Object(options)) = options else {
        return map;
    };
    let multisets = [
        "rstyle.compiler-advisory/1",
        "rstyle.compiler-generated-effects/2",
    ];
    let options = options.into_iter().map(|(key, value)| {
        let value = match multisets.contains(&key.as_str()) {
            true => sorted(value),
            false => value,
        };
        (format!("option:{key}"), value)
    });

    map.into_iter().chain(options).collect()
}

/// A value with every list in it sorted.
fn sorted(value: Value) -> Value {
    match value {
        Value::Array(items) => {
            let mut items: Vec<Value> = items.into_iter().map(sorted).collect();
            items.sort_by_key(|item| item.to_string());
            Value::Array(items)
        }
        Value::Object(fields) => fields
            .into_iter()
            .map(|(key, value)| (key, sorted(value)))
            .collect(),
        other => other,
    }
}
