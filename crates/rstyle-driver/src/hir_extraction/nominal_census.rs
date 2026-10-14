//! Capture original local compiler buffers for the selected data unit.

use super::{Options, original_source};
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
    let mut captured = local_buffers(tcx)?;
    captured.sort_by(|a, b| a.path.cmp(&b.path));

    let census = Census {
        identity: census::IDENTITY.into(),
        context: context.clone(),
        files: captured,
    };
    census::check_budget(&census)?;
    let value = serde_json::to_value(census).map_err(|e| e.to_string())?;
    for function in facts {
        function
            .build
            .frontend_options
            .insert(census::KEY.into(), value.clone());
    }
    Ok(())
}

fn local_buffers(tcx: TyCtxt<'_>) -> Result<Vec<UnitFile>, String> {
    tcx.sess
        .source_map()
        .files()
        .iter()
        .filter(|file| file.cnum == rustc_hir::def_id::LOCAL_CRATE)
        .filter_map(|file| match &file.name {
            rustc_span::FileName::Real(real) => {
                real.local_path().map(|path| original_file(tcx, file, path))
            }
            _ => None,
        })
        .collect()
}

fn original_file(
    tcx: TyCtxt<'_>,
    file: &rustc_span::SourceFile,
    path: &std::path::Path,
) -> Result<UnitFile, String> {
    let full = rustc_span::Span::with_root_ctxt(file.start_pos, file.end_position());
    let (mut bytes, span) =
        original_source(tcx, full).ok_or("Missing original local compiler source buffer")?;
    if span.start == 3 {
        bytes.insert(0, '\u{feff}');
    } else if span.start != 0 {
        return Err("Unexpected original source prefix".into());
    }
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };

    Ok(UnitFile {
        path,
        digest: rstyle_pdg::concrete_types::digest(bytes.as_bytes()),
        bytes,
    })
}
