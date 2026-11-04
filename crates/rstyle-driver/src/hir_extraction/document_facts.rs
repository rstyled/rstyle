//! The capture's frontend facts, read from the crate's `compat.cpg` documents.

use rstyle_pdg::HirFnFacts;
use rstyle_pdg::cpg_reader::{self, unit::Documents};
use rustc_middle::ty::TyCtxt;
use serde_json::json;
use std::collections::BTreeMap;

/// The facts of every function the crate owns, in owner order, read from its
/// documents, and what the documents record of the unit.
pub(super) fn capture(
    tcx: TyCtxt<'_>,
    (options, passes): (&super::Options<'_>, cpg_reader::Passes),
) -> Result<(Vec<HirFnFacts>, Documents), String> {
    let (mut derived, documents) = read_documents(tcx, (options, passes))?;

    let facts = super::function_owners(tcx)
        .map(|id| {
            let key = (tcx.def_path_str(id.to_def_id()), id.local_def_index.as_u32());
            derived
                .remove(&key)
                .unwrap_or_else(|| Err(format!("No document function {} at {}", key.0, key.1)))
                .map_err(|reason| format!("{}: {reason}", key.0))
        })
        .collect::<Result<_, _>>()?;
    Ok((facts, documents))
}

/// The facts read from the crate's documents, by function.
fn read_documents(
    tcx: TyCtxt<'_>,
    (options, passes): (&super::Options<'_>, cpg_reader::Passes),
) -> Result<(Derived, Documents), String> {
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
) -> Result<(Derived, Documents), String> {
    let profile = options.selected();
    let mut derived = BTreeMap::new();
    let mut documents = Documents::default();
    let observe = |id| super::function_observations(tcx, options, id);
    let capture = crate::cpg::Capture {
        session: Some(session),
        observe: Some(&observe),
        unit: unit_observations(tcx, options),
    };
    for (document, _) in crate::cpg::documents_in(tcx, (root, package), capture) {
        let document = rstyle_core::cpg::layout::bundle(vec![document])?;
        documents.add(&document);
        for read in cpg_reader::read(&document, profile, passes)? {
            derived.insert(read_key(&read), read.facts);
        }
    }

    Ok((derived, documents))
}

/// The unit's own compiler observations, as its extensions.
fn unit_observations(
    tcx: TyCtxt<'_>,
    options: &super::Options<'_>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut observed = serde_json::Map::new();
    if let Some(census) = super::nominal_census::observation(tcx, options) {
        observed.insert("rust:source-census".into(), census);
    }
    if let Some(namespaces) = super::nominal_census::namespace_observation(tcx, options) {
        observed.insert("rust:dependency-namespaces".into(), namespaces);
    }

    observed
}
