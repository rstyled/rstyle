//! Shared raw diagnostic transport. Decoding grants no verification authority.
use super::function_sources;
use crate::{HirFnFacts, SourceSnapshot, frontend::option_bundle, source_table};
use serde::{Deserialize, Serialize};
use std::io::Write;

pub const RAW_FACTS_SCHEMA: &str = "rstyle.diagnostic.raw-facts/5";

#[derive(Serialize)]
struct Export<'a, Function = function_sources::Export<'a>> {
    schema: &'static str,
    phase: &'a str,
    sources: Vec<&'a str>,
    inherent_units: option_bundle::Inventories<'a>,
    facts: Vec<Function>,
    #[serde(skip_serializing_if = "Option::is_none")]
    build_settings: Option<Vec<crate::BuildFacts<()>>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Import<
    B,
    Bindings = Vec<crate::BindingFact>,
    Build = crate::BuildFacts<option_bundle::Import>,
> {
    schema: String,
    phase: String,
    sources: Vec<SourceSnapshot>,
    inherent_units: Vec<serde_json::Value>,
    facts: Vec<function_sources::Import<B, Bindings, Build>>,
    #[serde(default)]
    build_settings: build_settings::Table,
}

/// Stream raw facts through the existing source-reference codec, without graphs.
/// This opt-in diagnostic format is not a complete publication or admission.
pub fn write_diagnostic_snapshot(
    writer: impl Write,
    phase: &str,
    facts: &[HirFnFacts],
) -> Result<(), String> {
    let mut settings = build_settings::Exporter::default();
    let encoded = function_sources::encode_with_build(
        facts,
        |facts| {
            (
                function_sources::body::Export(&facts.body),
                facts.bindings.as_slice(),
            )
        },
        option_bundle::Inventories::with_shared_options(),
        |build, sources, inventories| settings.reference(build, sources, inventories),
    )?;
    let export = Export {
        schema: RAW_FACTS_SCHEMA,
        phase,
        sources: encoded.snapshots,
        inherent_units: encoded.inventories,
        facts: encoded.functions,
        build_settings: Some(settings.settings),
    };
    serde_json::to_writer(writer, &export).map_err(|error| error.to_string())
}

/// Publish raw observations with the shared transport seal, without certification.
pub fn write_sealed_diagnostic_snapshot(
    writer: impl Write,
    phase: &str,
    facts: &[HirFnFacts],
) -> Result<super::super::BundleManifest, String> {
    super::super::manifest::seal(writer, |writer| {
        write_diagnostic_snapshot(writer, phase, facts)
    })
}

/// Restore raw evidence and validate local references only. No certificates,
/// helper summaries, graph pairs, or inherited admission permissions are loaded.
pub fn read_diagnostic_snapshot(bytes: &[u8]) -> Result<Vec<HirFnFacts>, String> {
    read_snapshot::<function_sources::FullBody>(bytes, None)
}

/// Restore observations only from the explicitly requested publication phase.
pub fn read_diagnostic_snapshot_phase(
    bytes: &[u8],
    expected: &str,
) -> Result<Vec<HirFnFacts>, String> {
    read_snapshot::<function_sources::FullBody>(bytes, Some(expected))
}

/// Read compiler metadata while validating and discarding original HIR payloads.
/// The body contains only its original root identity, with no fabricated tree.
pub fn read_diagnostic_snapshot_headers_phase(
    bytes: &[u8],
    expected: &str,
) -> Result<Vec<HirFnFacts<crate::HirRef>>, String> {
    read_snapshot::<function_sources::Headers>(bytes, Some(expected))
}

fn read_snapshot<B>(
    bytes: &[u8],
    expected: Option<&str>,
) -> Result<Vec<HirFnFacts<B::Output>>, String>
where
    B: function_sources::BodyImport + serde::de::DeserializeOwned,
{
    let wire: Import<B, Vec<crate::BindingFact>, build_settings::ImportedReference> =
        crate::diagnostics::Stage::new("diagnostic.parse", "snapshot", Some(bytes))
            .observe(|| serde_json::from_slice(bytes).map_err(|error| error.to_string()))?;
    crate::diagnostics::Stage::new("diagnostic.restore", "snapshot", None)
        .observe(|| restore_raw(wire, expected))
}

fn validate_raw_phase(schema: &str, phase: &str, expected: Option<&str>) -> Result<(), String> {
    if schema != RAW_FACTS_SCHEMA || phase.is_empty() {
        return Err("Unsupported or unlabelled raw diagnostic snapshot".into());
    }
    if expected.is_some_and(|expected| phase != expected) {
        return Err("Raw diagnostic publication phase disagrees with requested mode".into());
    }
    Ok(())
}

fn restore_snapshot<B: function_sources::BodyImport, Bindings>(
    wire: Import<B, Bindings>,
    flat_bodies: bool,
) -> Result<Vec<HirFnFacts<B::Output, Bindings>>, String> {
    if wire.build_settings.present() {
        return Err("Build settings references require raw diagnostic transport".into());
    }
    restore_with(wire, flat_bodies, Ok)
}

fn restore_with<B: function_sources::BodyImport, Bindings, Build>(
    wire: Import<B, Bindings, Build>,
    flat_bodies: bool,
    mut build: impl FnMut(Build) -> Result<crate::BuildFacts<option_bundle::Import>, String>,
) -> Result<Vec<HirFnFacts<B::Output, Bindings>>, String> {
    let mut inventories =
        option_bundle::ImportedInventories::new(Some(wire.inherent_units), false)?;
    inventories.share_censuses(true);
    inventories.share_options(true);
    let mut sources = source_table::Imported::new(&wire.sources);
    let facts = wire
        .facts
        .into_iter()
        .map(|function| {
            function.into_facts_with(
                function_sources::Decode {
                    sources: &mut sources,
                    inventories: &mut inventories,
                    flat_bodies,
                },
                &mut build,
            )
        })
        .collect::<Result<_, _>>()?;

    sources.finish()?;
    inventories.finish()?;
    Ok(facts)
}

fn restore_raw<B: function_sources::BodyImport>(
    mut wire: Import<B, Vec<crate::BindingFact>, build_settings::ImportedReference>,
    expected: Option<&str>,
) -> Result<Vec<HirFnFacts<B::Output>>, String> {
    validate_raw_phase(&wire.schema, &wire.phase, expected)?;

    let mut settings = std::mem::take(&mut wire.build_settings).import()?;
    let facts = restore_with(wire, true, |build| settings.restore(build))?;
    settings.finish()?;
    Ok(facts)
}

mod build_settings;

mod style;
pub use style::{STYLE_FACTS_SCHEMA, read_style_snapshot_phase, write_sealed_style_snapshot};

#[cfg(test)]
mod tests;
