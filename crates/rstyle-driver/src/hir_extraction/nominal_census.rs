//! Capture original local compiler buffers for the selected data unit.

use super::{Options, sources::Sources};
use rstyle_pdg::{
    HirFnFacts,
    nominal_boundary::census::{self, Census, UnitFile},
};
use rustc_middle::ty::TyCtxt;

// rstyle-allow(side_effect_density): the compiler census the unit diagnostic compares with
pub fn capture(
    tcx: TyCtxt<'_>,
    options: &Options<'_>,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    let Some(mut census) = file_census(tcx, options)? else {
        return Ok(());
    };
    let (imports, documentation, expansions) = parts(tcx, options)?;
    census.external_imports = imports;
    census.documentation = documentation;
    census.expansions = expansions;

    rstyle_pdg::cpg_reader::unit::retain(census, facts)
}

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

/// Reuse original buffers for local consumers without namespace or proof inputs.
pub fn capture_files(
    tcx: TyCtxt<'_>,
    options: &Options<'_>,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    let Some(census) = file_census(tcx, options)? else {
        return Ok(());
    };
    rstyle_pdg::cpg_reader::unit::retain(census, facts)
}

fn file_census(tcx: TyCtxt<'_>, options: &Options<'_>) -> Result<Option<Census>, String> {
    let Some(context) = options.unit_context else {
        return Ok(None);
    };
    let mut files = local_buffers(tcx, &options.sources)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Some(Census {
        identity: census::IDENTITY.into(),
        context: context.clone(),
        files,
        external_imports: Vec::new(),
        documentation: Vec::new(),
        expansions: Vec::new(),
    }))
}

fn local_buffers(tcx: TyCtxt<'_>, sources: &Sources) -> Result<Vec<UnitFile>, String> {
    tcx.sess
        .source_map()
        .files()
        .iter()
        .filter(|file| file.cnum == rustc_hir::def_id::LOCAL_CRATE)
        .filter_map(|file| match &file.name {
            rustc_span::FileName::Real(real) => real
                .local_path()
                .map(|path| original_file(sources, file, path)),
            _ => None,
        })
        .collect()
}

fn original_file(
    sources: &Sources,
    file: &rustc_span::SourceFile,
    path: &std::path::Path,
) -> Result<UnitFile, String> {
    let snapshot = sources
        .file(file)
        .ok_or("Missing original local compiler source buffer")?;
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };

    Ok(UnitFile {
        path,
        digest: snapshot.checksum().to_owned(),
        bytes: snapshot.bytes.clone(),
    })
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

pub fn namespaces(
    tcx: TyCtxt<'_>,
    options: &Options<'_>,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    if options.selected() == rstyle_pdg::extraction_profile::ExtractionProfile::DataV3
        && let Some(first) = facts.first_mut()
    {
        let namespaces = super::dependency_namespaces::capture(tcx, &options.sources)?;
        first
            .build
            .frontend_options
            .insert_data(std::sync::Arc::new(namespaces));
    }
    Ok(())
}
