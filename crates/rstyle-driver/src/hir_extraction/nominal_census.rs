//! Capture original local compiler buffers for the selected data unit.

use super::{Options, sources::Sources};
use rstyle_pdg::{
    HirFnFacts,
    nominal_boundary::census::{self, Census, UnitFile},
};
use rustc_middle::ty::TyCtxt;

pub fn capture(
    tcx: TyCtxt<'_>,
    options: &Options<'_>,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    let Some(mut census) = file_census(tcx, options)? else {
        return Ok(());
    };

    if options
        .selected()
        .supports(census::external_imports::CAPABILITY)
    {
        census.external_imports = super::external_imports::capture(tcx, &options.sources)?;
    }
    if options
        .selected()
        .supports(census::documentation::CAPABILITY)
    {
        census.documentation = super::documentation_inputs::capture(tcx, &options.sources)?;
    }
    if options.selected().supports("rust.cfg-if-module-ownership") {
        census.expansions = super::module_expansions::capture(tcx, &options.sources)?;
    }
    retain(census, facts)
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
    retain(census, facts)
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

fn retain(census: Census, facts: &mut [HirFnFacts]) -> Result<(), String> {
    census::check_budget(&census)?;
    let census = std::sync::Arc::new(census);
    for function in facts {
        function
            .build
            .frontend_options
            .insert_data(std::sync::Arc::clone(&census));
    }
    Ok(())
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
