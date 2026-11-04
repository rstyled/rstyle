//! The capture's frontend facts, read from the crate's `compat.cpg` documents.

use rstyle_pdg::HirFnFacts;
use rstyle_pdg::cpg_reader;
use rustc_middle::ty::TyCtxt;
use serde_json::json;
use std::collections::BTreeMap;

/// The facts of every function the crate owns, in owner order, read from its documents.
pub(super) fn capture(
    tcx: TyCtxt<'_>,
    (options, passes): (&super::Options<'_>, cpg_reader::Passes),
) -> Result<Vec<HirFnFacts>, String> {
    let mut derived = read_documents(tcx, (options, passes))?;

    super::function_owners(tcx)
        .map(|id| {
            let key = (tcx.def_path_str(id.to_def_id()), id.local_def_index.as_u32());
            derived
                .remove(&key)
                .unwrap_or_else(|| Err(format!("No document function {} at {}", key.0, key.1)))
                .map_err(|reason| format!("{}: {reason}", key.0))
        })
        .collect()
}

/// The facts read from the crate's documents, by function.
fn read_documents(
    tcx: TyCtxt<'_>,
    (options, passes): (&super::Options<'_>, cpg_reader::Passes),
) -> Result<Derived, String> {
    let root = document_root(tcx, options);
    let package = options.unit_context.map(|context| &context.package);
    let session = json!({"invocation": options.invocation,
                         "profile_selection": options.profile.map(|profile| profile.token()),
                         "selected_unit": options.unit_context});
    let root = root.ok_or_else(|| "The crate has no root source file".to_owned())?;

    derived(tcx, (root.as_path(), package, session), (options, passes))
}

/// The root a capture's paths are relative to, as its advisory facts locate them: the
/// source root that holds the selected package, else the package's own directory;
/// without either, the crate root file's directory.
fn document_root(tcx: TyCtxt<'_>, options: &super::Options<'_>) -> Option<std::path::PathBuf> {
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
    rstyle_pdg::nominal_binding::absolute_path(directory).ok()
}

type Key = (String, u32);
type Derived = BTreeMap<Key, Result<HirFnFacts, String>>;

fn read_key(read: &cpg_reader::Read) -> Key {
    (read.name.clone(), read.owner)
}

fn derived(
    tcx: TyCtxt<'_>,
    (root, package, session): (
        &std::path::Path,
        Option<&rstyle_pdg::nominal_binding::PackageContext>,
        serde_json::Value,
    ),
    (options, passes): (&super::Options<'_>, cpg_reader::Passes),
) -> Result<Derived, String> {
    let profile = options.selected();
    let mut derived = BTreeMap::new();
    let observe = |id| super::function_observations(tcx, options, id);
    let observe: &crate::cpg::Observe<'_> = &observe;
    for (document, _) in
        crate::cpg::documents_in(tcx, (root, package), (Some(session.clone()), Some(observe)))
    {
        let document = rstyle_core::cpg::layout::bundle(vec![document])?;
        for read in cpg_reader::read(&document, profile, passes)? {
            derived.insert(read_key(&read), read.facts);
        }
    }

    Ok(derived)
}
