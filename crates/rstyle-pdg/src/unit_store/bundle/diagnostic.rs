//! Shared raw diagnostic transport. Decoding grants no verification authority.
use super::function_sources;
use crate::{HirFnFacts, SourceSnapshot, frontend::option_bundle, source_table};
use serde::{Deserialize, Serialize};
use std::io::Write;

const SCHEMA: &str = "rstyle.diagnostic.raw-facts/3";

#[derive(Serialize)]
struct Export<'a> {
    schema: &'static str,
    phase: &'a str,
    sources: Vec<&'a str>,
    inherent_units: option_bundle::Inventories<'a>,
    facts: Vec<function_sources::Export<'a>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Import {
    schema: String,
    phase: String,
    sources: Vec<SourceSnapshot>,
    inherent_units: Vec<serde_json::Value>,
    facts: Vec<function_sources::Import>,
}

/// Stream raw facts through the existing source-reference codec, without graphs.
/// This opt-in diagnostic format is not a complete publication or admission.
pub fn write_diagnostic_snapshot(
    writer: impl Write,
    phase: &str,
    facts: &[HirFnFacts],
) -> Result<(), String> {
    let encoded = function_sources::encode(facts)?;
    let export = Export {
        schema: SCHEMA,
        phase,
        sources: encoded.snapshots,
        inherent_units: encoded.inventories,
        facts: encoded.functions,
    };
    serde_json::to_writer(writer, &export).map_err(|error| error.to_string())
}

/// Restore raw evidence and validate local references only. No certificates,
/// helper summaries, graph pairs, or inherited admission permissions are loaded.
pub fn read_diagnostic_snapshot(bytes: &[u8]) -> Result<Vec<HirFnFacts>, String> {
    let wire: Import = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    if ![SCHEMA, "rstyle.diagnostic.raw-facts/2"].contains(&wire.schema.as_str())
        || wire.phase.is_empty()
    {
        return Err("Unsupported or unlabelled raw diagnostic snapshot".into());
    }

    let mut inventories =
        option_bundle::ImportedInventories::new(Some(wire.inherent_units), false)?;
    let mut sources = source_table::Imported::new(&wire.sources);
    let flat_bodies = wire.schema == SCHEMA;
    let facts = wire
        .facts
        .into_iter()
        .map(|function| function.into_facts(&mut sources, &mut inventories, flat_bodies))
        .collect::<Result<_, _>>()?;

    sources.finish()?;
    inventories.finish()?;
    Ok(facts)
}

#[cfg(test)]
mod tests;
