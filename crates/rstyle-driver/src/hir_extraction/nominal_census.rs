//! The selected unit's source census and dependency namespaces, as the compiler
//! observes them, recorded as the unit's document extensions.

use super::Options;
use rstyle_pdg::nominal_boundary::census;
use rustc_middle::ty::TyCtxt;

type Parts = (
    Vec<census::external_imports::Import>,
    Vec<census::documentation::Input>,
    Vec<census::expansions::ModuleExpansion>,
);

/// The census's compiler observations the selected profile admits: macro imports of
/// external crates, builtin documentation inputs and `cfg_if` module ownership.
fn parts(tcx: TyCtxt<'_>, options: &Options<'_>) -> Result<Parts, String> {
    let profile = options.selected();
    let imports = match profile.supports(census::external_imports::CAPABILITY) {
        true => super::external_imports::capture(tcx, &options.sources)?,
        false => Vec::new(),
    };
    let documentation = match profile.supports(census::documentation::CAPABILITY) {
        true => super::documentation_inputs::capture(tcx, &options.sources)?,
        false => Vec::new(),
    };
    let expansions = match profile.supports("rust.cfg-if-module-ownership") {
        true => super::module_expansions::capture(tcx, &options.sources)?,
        false => Vec::new(),
    };

    Ok((imports, documentation, expansions))
}

// rstyle-allow(side_effect_density): records the compiler's census observations, or why they failed
/// The census's compiler observations as the unit's `rust:source-census` extension, or
/// why they could not be observed; none outside a selected unit.
pub(super) fn observation(tcx: TyCtxt<'_>, options: &Options<'_>) -> Option<serde_json::Value> {
    options.unit_context?;
    let observed = parts(tcx, options).and_then(|(imports, documentation, expansions)| {
        serde_json::to_value(census::Observations {
            external_imports: imports,
            documentation,
            expansions,
        })
        .map_err(|error| error.to_string())
    });

    Some(observed.unwrap_or_else(|error| serde_json::json!({"error": error})))
}

/// The unit's dependency namespaces as the `rust:dependency-namespaces` extension, or
/// why they could not be observed; none outside the data profile that reads them.
pub(super) fn namespace_observation(
    tcx: TyCtxt<'_>,
    options: &Options<'_>,
) -> Option<serde_json::Value> {
    (options.selected() == rstyle_pdg::extraction_profile::ExtractionProfile::DataV3)
        .then_some(())?;
    let observed = super::dependency_namespaces::capture(tcx, &options.sources)
        .and_then(|namespaces| serde_json::to_value(namespaces).map_err(|e| e.to_string()));

    Some(observed.unwrap_or_else(|error| serde_json::json!({"error": error})))
}
