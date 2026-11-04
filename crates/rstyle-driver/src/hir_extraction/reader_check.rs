//! Diagnostic: the frontend facts read back from the crate's `compat.cpg` documents,
//! compared channel by channel with the facts this capture produced. Enabled by
//! `RSTYLE_DIAGNOSTIC_CPG_READER_DIR`, which receives one JSON-lines file per crate.

use rstyle_pdg::HirFnFacts;
use rstyle_pdg::cpg_reader::{self, compare::Difference};
use rustc_middle::ty::TyCtxt;
use serde_json::json;
use std::collections::BTreeMap;
use std::io::Write;

pub const DIR_ENV: &str = "RSTYLE_DIAGNOSTIC_CPG_READER_DIR";

/// With `READ_ENV` set, the capture's facts are the ones read from its documents.
pub const READ_ENV: &str = "RSTYLE_CPG_READER_FACTS";

/// Compares the capture's facts with those read from its documents when asked, and
/// returns the facts the capture continues with: its own, or with `READ_ENV` set the
/// facts read back, which must then cover every function.
pub(super) fn check(
    tcx: TyCtxt<'_>,
    (options, passes): (&super::Options<'_>, cpg_reader::Passes),
    facts: Vec<HirFnFacts>,
) -> Result<Vec<HirFnFacts>, String> {
    let dir = std::env::var_os(DIR_ENV);
    let read = std::env::var_os(READ_ENV).is_some();
    if dir.is_none() && !read {
        return Ok(facts);
    }
    let derived = read_documents(tcx, (options, passes), &facts);
    if let Some(dir) = dir {
        let profile = options.selected().token();
        let lines = facts.iter().map(|reference| {
            let differences = function_differences(reference, &derived);
            json!({"function": reference.name, "profile": profile, "differences": differences})
        });
        write(tcx, std::path::Path::new(&dir), lines)?;
    }
    if !read {
        return Ok(facts);
    }
    let mut derived = derived;

    facts
        .iter()
        .map(|reference| {
            let key = (reference.name.clone(), reference.fn_span.start);
            derived
                .remove(&key)
                .unwrap_or_else(|| Err(format!("No document function {} at {}", key.0, key.1)))
                .map_err(|reason| format!("{}: {reason}", reference.name))
        })
        .collect()
}

/// The facts read from the crate's documents, by function; a failure to read the
/// documents is each function's.
fn read_documents(
    tcx: TyCtxt<'_>,
    (options, passes): (&super::Options<'_>, cpg_reader::Passes),
    facts: &[HirFnFacts],
) -> Derived {
    let root = document_root(tcx, options);
    let package = options.unit_context.map(|context| &context.package);
    let session = json!({"invocation": options.invocation,
                         "profile_selection": options.profile.map(|profile| profile.token()),
                         "selected_unit": options.unit_context});
    let result = root
        .ok_or_else(|| "The crate has no root source file".to_owned())
        .and_then(|root| derived(tcx, (root.as_path(), package, session), (options, passes)));

    result.unwrap_or_else(|error| {
        facts
            .iter()
            .map(|reference| {
                (
                    (reference.name.clone(), reference.fn_span.start),
                    Err(error.clone()),
                )
            })
            .collect()
    })
}

/// One function's differences from the facts read for it, or why none were read.
fn function_differences(reference: &HirFnFacts, derived: &Derived) -> Vec<Difference> {
    let key = (reference.name.clone(), reference.fn_span.start);
    match derived.get(&key) {
        Some(Ok(derived)) => cpg_reader::compare::differences(reference, derived),
        Some(Err(reason)) => vec![unread("function.read", reason)],
        None => {
            let starts: Vec<u32> = derived
                .keys()
                .filter(|(name, _)| *name == reference.name)
                .map(|(_, start)| *start)
                .collect();
            let reason = match starts.as_slice() {
                [] => "No document function has this name".to_owned(),
                starts => format!(
                    "Document functions of this name start at {starts:?}, not {}",
                    key.1
                ),
            };
            vec![unread("function.absent", &reason)]
        }
    }
}

// rstyle-allow(side_effect_density): diagnostic boundary: writes the comparison it was asked for
fn write(
    tcx: TyCtxt<'_>,
    dir: &std::path::Path,
    lines: impl Iterator<Item = serde_json::Value>,
) -> Result<(), String> {
    let name = format!(
        "{}-{:016x}-{}.jsonl",
        tcx.crate_name(rustc_span::def_id::LOCAL_CRATE),
        tcx.stable_crate_id(rustc_span::def_id::LOCAL_CRATE)
            .as_u64(),
        std::process::id()
    );
    let path = dir.join(name);
    let mut file =
        std::fs::File::create(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    for line in lines {
        writeln!(file, "{line}").map_err(|error| error.to_string())?;
    }

    Ok(())
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

type Derived = BTreeMap<(String, u32), Result<HirFnFacts, String>>;

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
            derived.insert((read.name, read.fn_span.start), read.facts);
        }
    }

    Ok(derived)
}

fn unread(channel: &str, reason: &str) -> Difference {
    Difference {
        channel: channel.into(),
        path: String::new(),
        reference: "present".into(),
        derived: reason.into(),
    }
}
