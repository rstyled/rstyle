//! Publishes a crate's documents and the manifest sealing them into its unit directory.
use rstyle_capture::UnitContext;
use rstyle_core::cpg::transport::{DOCUMENTS_MANIFEST, DocumentManifest};
use rustc_middle::ty::TyCtxt;
use serde_json::json;
use std::path::Path;

/// What this compilation was asked to capture.
pub struct Options<'a> {
    pub source_root: Option<&'a Path>,
    pub invocation: &'a [String],
    pub unit_context: Option<&'a UnitContext>,
}

fn session(options: &Options<'_>) -> serde_json::Value {
    json!({"invocation": options.invocation, "selected_unit": options.unit_context})
}

// rstyle-allow(side_effect_density): IO boundary: writes each document, then the seal over them
/// Writes the crate's documents into `directory`, then the manifest that seals them.
pub fn publish(
    tcx: TyCtxt<'_>,
    options: &Options<'_>,
    directory: &std::path::Path,
) -> Result<(), String> {
    let root = document_root(tcx, options)
        .ok_or_else(|| "The crate has no root source file".to_owned())?;
    let package = options.unit_context.map(|context| &context.package);

    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let encoding = rstyle_core::cpg::transport::Encoding::from_environment()?;
    let mut sealed = Vec::new();
    for (document, unrecorded) in
        super::documents_in(tcx, (root.as_path(), package), Some(session(options)))
    {
        if let Some((id, reason)) = unrecorded.first() {
            return Err(format!("{}: {reason}", tcx.def_path_str(id.to_def_id())));
        }
        let path = directory.join(format!("document-{:06}{}", sealed.len(), encoding.suffix()));
        let seal = rstyle_core::cpg::transport::write_json(&path, encoding, &document)?;
        sealed.push(seal);
    }

    let bytes =
        serde_json::to_vec(&DocumentManifest::of(sealed)).map_err(|error| error.to_string())?;
    std::fs::write(directory.join(DOCUMENTS_MANIFEST), bytes).map_err(|error| error.to_string())
}

/// The root a capture's paths are relative to, as its advisory facts locate them: the
/// source root that holds the selected package, else the package's own directory;
/// without either, the crate root file's directory.
fn document_root(tcx: TyCtxt<'_>, options: &Options<'_>) -> Option<std::path::PathBuf> {
    let package = options
        .unit_context
        .and_then(|context| context.package.manifest_path.parent());
    match (options.source_root, package) {
        (Some(root), Some(package)) if !package.starts_with(root) => Some(package.to_path_buf()),
        (Some(root), _) => Some(root.to_path_buf()),
        (None, Some(package)) => Some(package.to_path_buf()),
        (None, None) => crate_directory(tcx),
    }
}

fn crate_directory(tcx: TyCtxt<'_>) -> Option<std::path::PathBuf> {
    let file = tcx.sess.local_crate_source_file()?;
    let path = std::path::absolute(file.local_path()?).ok()?;
    let directory = path.parent()?;
    rstyle_capture::absolute_path(directory).ok()
}
