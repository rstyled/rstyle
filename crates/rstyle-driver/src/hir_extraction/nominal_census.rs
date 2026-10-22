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
    let Some(context) = options.unit_context else {
        return Ok(());
    };
    let mut captured = local_buffers(tcx, &options.sources)?;
    captured.sort_by(|a, b| a.path.cmp(&b.path));

    let census = Census {
        identity: census::IDENTITY.into(),
        context: context.clone(),
        files: captured,
        external_imports: if options
            .selected()
            .supports(census::external_imports::CAPABILITY)
        {
            super::external_imports::capture(tcx, &options.sources)?
        } else {
            Vec::new()
        },
        documentation: if options
            .selected()
            .supports(census::documentation::CAPABILITY)
        {
            super::documentation_inputs::capture(tcx, &options.sources)?
        } else {
            Vec::new()
        },
        expansions: if options.selected().supports("rust.cfg-if-module-ownership") {
            super::module_expansions::capture(tcx, &options.sources)?
        } else {
            Vec::new()
        },
    };
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
        digest: snapshot.identity().to_owned(),
        bytes: snapshot.bytes.clone(),
    })
}
