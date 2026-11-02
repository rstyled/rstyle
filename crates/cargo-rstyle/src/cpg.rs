//! `cargo rstyle cpg check`: the style rules over `compat.cpg` documents from any
//! language frontend, without compiling anything.

use rstyle_core::config::Config;
use rstyle_core::report;
use std::path::{Path, PathBuf};

use crate::Format;

/// Style report of one bundled document or a split run, and its exit code.
pub fn check(
    documents: &[PathBuf],
    config: Option<&Path>,
    format: Format,
) -> Result<(String, u8), String> {
    let values = documents
        .iter()
        .map(|path| read(path))
        .collect::<Result<Vec<_>, _>>()?;
    let facts = rstyle_core::cpg::run_facts(values)?;

    let report = rstyle_core::evaluate(facts, &load_config(config)?)?;
    let output = match format {
        Format::Text => report::text(&report),
        Format::Json => report::json(&report).map_err(|error| error.to_string())?,
    };
    Ok((output, report.exit_code()))
}

// rstyle-allow(side_effect_density): IO boundary: reads one input document
fn read(path: &Path) -> Result<serde_json::Value, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
}

/// Documents name no Cargo workspace, so only an explicit `--config` applies.
// rstyle-allow(side_effect_density): IO boundary: reads the explicit config file
fn load_config(path: Option<&Path>) -> Result<Config, String> {
    let Some(path) = path else {
        return Ok(Config::default());
    };
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    Config::parse(&text).map_err(|error| format!("{}: {error}", path.display()))
}
