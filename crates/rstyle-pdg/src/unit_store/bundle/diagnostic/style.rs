//! Root style inputs retain source ownership and compiler metadata without body payloads.
use super::*;
use crate::{HirRef, StyleFnFacts, Uncaptured};

pub const STYLE_FACTS_SCHEMA: &str = "rstyle.diagnostic.style-facts/2";

/// Publish numeric observations with explicit omitted bindings and an actual body identity.
// rstyle-allow(side_effect_density): Style snapshot publication streams encoded facts through the existing sealed writer boundary.
pub fn write_sealed_style_snapshot(
    writer: impl Write,
    phase: &str,
    facts: &[StyleFnFacts],
) -> Result<super::super::super::BundleManifest, String> {
    super::super::super::manifest::seal(writer, |writer| {
        let encoded = function_sources::encode_with(
            facts,
            |facts| (&facts.body, &facts.bindings),
            option_bundle::Inventories::with_shared_options(),
        )?;
        let export = Export {
            schema: STYLE_FACTS_SCHEMA,
            phase,
            sources: encoded.snapshots,
            inherent_units: encoded.inventories,
            facts: encoded.functions,
        };
        serde_json::to_writer(writer, &export).map_err(|error| error.to_string())
    })
}

/// Restore only the explicitly requested root-style publication shape and phase.
// rstyle-allow(side_effect_density): Snapshot decoding has no external IO; its inherited IO and clock events are optional stage telemetry.
pub fn read_style_snapshot_phase(
    bytes: &[u8],
    expected: &str,
) -> Result<Vec<StyleFnFacts>, String> {
    let parse =
        crate::diagnostics::Stage::new("diagnostic.parse", "style-snapshot", Some(bytes)).enter();
    let wire: Import<HirRef, Uncaptured> =
        parse.finish(serde_json::from_slice(bytes).map_err(|error| error.to_string()))?;

    let restore =
        crate::diagnostics::Stage::new("diagnostic.restore", "style-snapshot", None).enter();
    if ![STYLE_FACTS_SCHEMA, "rstyle.diagnostic.style-facts/1"].contains(&wire.schema.as_str())
        || wire.phase.is_empty()
        || wire.phase != expected
    {
        return Err("Unsupported or mismatched root style snapshot schema or phase".into());
    }
    let shared_options = wire.schema == STYLE_FACTS_SCHEMA;
    restore.finish(restore_snapshot(wire, true, false, shared_options))
}

#[cfg(test)]
mod tests;
