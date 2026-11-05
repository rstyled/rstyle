//! Where a local source file sits relative to the workspace root.

use rustc_middle::ty::TyCtxt;
use rustc_span::{FileName, SourceFile};
use std::path::Path;

/// A local file's path relative to `root`, climbing out of it with `..` for a file
/// outside it, as a document records every file a unit compiles.
pub(crate) fn document_path(tcx: TyCtxt<'_>, file: &SourceFile, root: &Path) -> Option<String> {
    if let Some(path) = source_path(tcx, file, root) {
        return Some(path);
    }
    let FileName::Real(real) = &file.name else {
        return None;
    };
    let absolute = tcx
        .sess
        .source_map()
        .working_dir()
        .local_path()?
        .join(real.local_path()?);
    let normalized = rstyle_pdg::nominal_binding::absolute_path(&absolute).ok()?;
    let common = root
        .components()
        .zip(normalized.components())
        .take_while(|(left, right)| left == right)
        .count();
    let ups = root.components().count() - common;
    let rest = normalized
        .components()
        .skip(common)
        .map(|part| part.as_os_str().to_string_lossy().into_owned());
    let parts: Vec<String> = std::iter::repeat_n("..".to_owned(), ups)
        .chain(rest)
        .collect();

    Some(parts.join("/"))
}

pub(crate) fn source_path(tcx: TyCtxt<'_>, file: &SourceFile, root: &Path) -> Option<String> {
    let FileName::Real(real) = &file.name else {
        return None;
    };

    let path = real.local_path()?;
    let absolute = tcx.sess.source_map().working_dir().local_path()?.join(path);
    let normalized = rstyle_pdg::nominal_binding::absolute_path(&absolute).ok()?;
    let relative = normalized.strip_prefix(root).ok()?;
    Some(relative.to_string_lossy().replace('\\', "/"))
}
