//! The capture's frontend facts, read from the crate's `compat.cpg` documents.

use rstyle_core::advisory::runtime_bounds::{CAPTURE_ENV, Input, KEY, Observation};
use rstyle_pdg::HirFnFacts;
use rstyle_pdg::cpg_reader::{self, unit::Documents};
use rstyle_pdg::unit_store::{DOCUMENTS_MANIFEST, DocumentManifest};
use rustc_middle::ty::TyCtxt;
use serde_json::json;
use std::collections::BTreeMap;

/// The facts of every function the crate owns, in owner order, read from its
/// documents, and what the documents record of the unit.
pub(super) fn capture(
    tcx: TyCtxt<'_>,
    (options, passes): (&super::Options<'_>, cpg_reader::Passes),
) -> Result<(Vec<HirFnFacts>, Documents), String> {
    let Read {
        facts: mut derived,
        documents,
        programs,
    } = read_documents(tcx, (options, passes))?;

    let mut facts: Vec<HirFnFacts> = super::function_owners(tcx)
        .map(|id| {
            let key = (
                tcx.def_path_str(id.to_def_id()),
                id.local_def_index.as_u32(),
            );
            derived
                .remove(&key)
                .unwrap_or_else(|| Err(format!("No document function {} at {}", key.0, key.1)))
                .map_err(|reason| format!("{}: {reason}", key.0))
        })
        .collect::<Result<_, _>>()?;
    if let Some(programs) = programs {
        bounds(tcx, options, &programs, &mut facts)?;
    }

    Ok((facts, documents))
}

/// What the crate's documents give the capture: each function's facts, what they
/// record of the unit and, when the run asks for them, each owner's runtime-bound
/// program as the core lowers it, or why lowering failed.
struct Read {
    facts: Derived,
    documents: Documents,
    programs: Option<Result<BTreeMap<String, Input>, String>>,
}

/// Each function's runtime-bound observation: its program, or why it has none.
fn bounds(
    tcx: TyCtxt<'_>,
    options: &super::Options<'_>,
    programs: &Result<BTreeMap<String, Input>, String>,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    let krate = format!(
        "{:016x}",
        tcx.stable_crate_id(rustc_span::def_id::LOCAL_CRATE)
            .as_u64()
    );
    for (id, function) in super::function_owners(tcx).zip(facts.iter_mut()) {
        let owner = crate::ids::def_id_str(tcx, id.to_def_id());
        let unavailable = |reason: String| Input::Unavailable { reason };
        let input = match (options.source_root, programs) {
            (None, _) => unavailable("Advisory capture has no source root".into()),
            (Some(_), Err(error)) => unavailable(format!("compat.cpg lowering failed: {error}")),
            (Some(_), Ok(programs)) => programs.get(&owner).cloned().unwrap_or_else(|| {
                unavailable("Owner is absent from the crate's compat.cpg document".into())
            }),
        };
        let observation = Observation {
            schema: KEY.into(),
            owner,
            krate: krate.clone(),
            input,
        };
        let value = serde_json::to_value(observation).map_err(|error| error.to_string())?;
        function.build.frontend_options.insert(KEY.into(), value);
    }

    Ok(())
}

/// The facts read from the crate's documents, by function.
fn read_documents(
    tcx: TyCtxt<'_>,
    (options, passes): (&super::Options<'_>, cpg_reader::Passes),
) -> Result<Read, String> {
    let root = document_root(tcx, options);
    let package = options.unit_context.map(|context| &context.package);
    let root = root.ok_or_else(|| "The crate has no root source file".to_owned())?;

    derived(
        tcx,
        (root.as_path(), package, session(options)),
        (options, passes),
    )
}

/// What a capture's documents record of the compiler session that wrote them.
fn session(options: &super::Options<'_>) -> serde_json::Value {
    json!({"invocation": options.invocation,
           "profile_selection": options.profile.map(|profile| profile.token()),
           "selected_unit": options.unit_context})
}

// rstyle-allow(side_effect_density): IO boundary: writes each document, then the seal over them
/// Writes the crate's documents into `directory`, then the manifest that seals them.
pub(super) fn publish(
    tcx: TyCtxt<'_>,
    options: &super::Options<'_>,
    directory: &std::path::Path,
) -> Result<(), String> {
    let root = document_root(tcx, options)
        .ok_or_else(|| "The crate has no root source file".to_owned())?;
    let package = options.unit_context.map(|context| &context.package);
    let observe = |id| super::function_observations(tcx, options, id);
    let capture = crate::cpg::Capture {
        session: Some(session(options)),
        observe: Some(&observe),
        unit: unit_observations(tcx, options),
    };

    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let encoding = rstyle_core::cpg::transport::Encoding::from_environment()?;
    let mut sealed = Vec::new();
    for (document, unrecorded) in crate::cpg::documents_in(tcx, (root.as_path(), package), capture)
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
) -> Result<Read, String> {
    let profile = options.selected();
    let mut derived = BTreeMap::new();
    let mut documents = Documents::default();
    let requested = std::env::var_os(CAPTURE_ENV).as_deref() == Some(std::ffi::OsStr::new("1"));
    let mut programs = requested.then(|| Ok(BTreeMap::new()));
    let observe = |id| super::function_observations(tcx, options, id);
    let capture = crate::cpg::Capture {
        session: Some(session),
        observe: Some(&observe),
        unit: unit_observations(tcx, options),
    };
    for (document, unrecorded) in crate::cpg::documents_in(tcx, (root, package), capture) {
        if let Some(programs) = programs.as_mut() {
            lower(tcx, programs, (&document, unrecorded));
        }
        let document = rstyle_core::cpg::layout::bundle(vec![document])?;
        documents.add(&document);
        for read in cpg_reader::read(&document, profile, passes)? {
            derived.insert(read_key(&read), read.facts);
        }
    }

    Ok(Read {
        facts: derived,
        documents,
        programs,
    })
}

// rstyle-allow(mutation_per_target): extends the crate's programs with one document's, or keeps why lowering failed
/// Extends the crate's runtime-bound programs with one document's: each function it
/// records, lowered by the core, and each it could not record, with why.
fn lower(
    tcx: TyCtxt<'_>,
    programs: &mut Result<BTreeMap<String, Input>, String>,
    (document, unrecorded): (
        &serde_json::Value,
        Vec<(rustc_hir::def_id::LocalDefId, String)>,
    ),
) {
    let Ok(lowered) = programs else {
        return;
    };
    let unavailable = unrecorded.into_iter().map(|(id, reason)| {
        let owner = crate::ids::def_id_str(tcx, id.to_def_id());
        (owner, Input::Unavailable { reason })
    });
    lowered.extend(unavailable);
    match rstyle_core::cpg::run_bounds(vec![document.clone()]) {
        Ok(bounds) => lowered.extend(bounds),
        Err(error) => *programs = Err(error),
    }
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
    if let Some(generated) = super::generated_sources::observe::unit(tcx, options) {
        let name = cpg_reader::generated_sources::UNIT_EXTENSION;
        observed.insert(name.into(), generated);
    }

    observed
}
