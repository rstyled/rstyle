//! Diagnostic: instance projections rebuilt from the documents, compared with the
//! compiler pass they replace. `RSTYLE_DIAGNOSTIC_PROJECTION_DIR` receives one
//! JSON-lines file per crate naming each function's differing frontend options.

use rstyle_pdg::HirFnFacts;
use rustc_middle::ty::TyCtxt;
use serde_json::{Value, json};
use std::io::Write;

pub const DIR_ENV: &str = "RSTYLE_DIAGNOSTIC_PROJECTION_DIR";

/// The facts the compiler `passes` produce from `facts`, when a comparison is requested.
pub(super) fn reference(
    facts: &[HirFnFacts],
    passes: impl FnOnce(&mut [HirFnFacts]) -> Result<(), String>,
) -> Option<Result<Vec<HirFnFacts>, String>> {
    std::env::var_os(DIR_ENV)?;
    let mut reference = facts.to_vec();

    Some(passes(&mut reference).map(|()| reference))
}

// rstyle-allow(side_effect_density): diagnostic boundary: writes the comparison it was asked for
/// Writes each function's frontend options that differ from the reference's.
pub(super) fn compare(
    tcx: TyCtxt<'_>,
    reference: Option<Result<Vec<HirFnFacts>, String>>,
    derived: &[HirFnFacts],
) -> Result<(), String> {
    let (Some(reference), Some(dir)) = (reference, std::env::var_os(DIR_ENV)) else {
        return Ok(());
    };
    let lines: Vec<Value> = match reference {
        Err(error) => vec![json!({"function": null, "reference_error": error})],
        Ok(reference) => reference
            .iter()
            .zip(derived)
            .map(|(reference, derived)| {
                json!({"function": reference.name, "differences": differences(reference, derived),
                       "present": present(derived)})
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

/// The projection options a function's facts carry.
fn present(facts: &HirFnFacts) -> Vec<&'static str> {
    [
        "rstyle.rust.scalar-instance-projections",
        "rstyle.rust.shared-slice-instance-projections",
    ]
    .into_iter()
    .filter(|key| facts.build.frontend_options.contains_key(key))
    .collect()
}

/// The option keys whose values differ, with both values.
fn differences(reference: &HirFnFacts, derived: &HirFnFacts) -> Vec<Value> {
    let options = |facts: &HirFnFacts| match serde_json::to_value(&facts.build.frontend_options) {
        Ok(Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    };
    let (reference, derived) = (options(reference), options(derived));
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
