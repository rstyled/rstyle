//! The frontend's findings on a function's own source before expansion, as its
//! document records its pre-expansion scan.

use super::Context;
use crate::frontend::HirDiagnostic;
use serde::Deserialize;

/// The function extension recording a function's pre-expansion source scan.
pub const EXTENSION: &str = "rust:source-scan";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scan {
    findings: Vec<HirDiagnostic>,
    complete: bool,
}

impl Context<'_, '_> {
    /// The findings on the function's source, and whether all of it was scanned.
    pub(super) fn source_scan(&self) -> Result<(Vec<HirDiagnostic>, bool), String> {
        let recorded = self
            .function_extension(EXTENSION)?
            .ok_or("The function records no source scan")?;
        let scan: Scan =
            serde_json::from_value(recorded.clone()).map_err(|error| error.to_string())?;

        Ok((scan.findings, scan.complete))
    }
}
