//! The capture's frontend facts, read from the crate's `compat.cpg` documents. The
//! compiler capture these documents replace remains a diagnostic oracle:
//! `RSTYLE_DIAGNOSTIC_CPG_READER_DIR` receives, per crate, one JSON-lines file comparing
//! it channel by channel with the facts read, and `RSTYLE_DIAGNOSTIC_DIRECT_CAPTURE`
//! continues with its facts instead.

use rstyle_pdg::HirFnFacts;
use rstyle_pdg::cpg_reader::{self, compare::Difference};
use rustc_middle::ty::TyCtxt;
use serde_json::json;
use std::collections::BTreeMap;
use std::io::Write;

pub const DIR_ENV: &str = "RSTYLE_DIAGNOSTIC_CPG_READER_DIR";

/// With `DIRECT_ENV` set, the capture continues with the compiler capture's own facts.
pub const DIRECT_ENV: &str = "RSTYLE_DIAGNOSTIC_DIRECT_CAPTURE";

/// The facts of every function the crate owns, in owner order, read from its documents;
/// with `DIRECT_ENV` set, the `direct` capture's. With `DIR_ENV` set, the two are
/// compared first.
pub(super) fn capture(
    tcx: TyCtxt<'_>,
    (options, passes): (&super::Options<'_>, cpg_reader::Passes),
    direct: impl FnOnce() -> Result<Vec<HirFnFacts>, String>,
) -> Result<Vec<HirFnFacts>, String> {
    let dir = std::env::var_os(DIR_ENV);
    let continue_direct = std::env::var_os(DIRECT_ENV).is_some();
    if continue_direct && dir.is_none() {
        return direct();
    }
    let derived = read_documents(tcx, (options, passes));
    if let Some(dir) = dir {
        let facts = direct()?;
        compare(tcx, (options, std::path::Path::new(&dir)), &facts, &derived)?;
        if continue_direct {
            return Ok(facts);
        }
    }
    let mut derived = derived?;

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

/// Writes each captured function's differences from the facts read for it.
fn compare(
    tcx: TyCtxt<'_>,
    (options, dir): (&super::Options<'_>, &std::path::Path),
    facts: &[HirFnFacts],
    derived: &Result<Derived, String>,
) -> Result<(), String> {
    let profile = options.selected().token();
    let lines = facts.iter().map(|reference| {
        let differences = match derived {
            Ok(derived) => function_differences(reference, derived),
            Err(error) => vec![unread("function.read", error)],
        };
        let mut line = json!({"function": reference.name, "profile": profile});
        if !differences.is_empty() {
            line["source"] = json!(reference.source.bytes);
        }
        line["differences"] = json!(differences);
        line
    });
    write(tcx, dir, lines)
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

/// One function's differences from the facts read for it, or why none were read.
fn function_differences(reference: &HirFnFacts, derived: &Derived) -> Vec<Difference> {
    let key = key(reference);
    match derived.get(&key) {
        Some(Ok(derived)) => cpg_reader::compare::differences(reference, derived),
        Some(Err(reason)) => vec![unread("function.read", reason)],
        None => {
            let starts: Vec<u32> = derived
                .keys()
                .filter(|(name, _)| *name == reference.name)
                .map(|(_, owner)| *owner)
                .collect();
            let reason = match starts.as_slice() {
                [] => "No document function has this name".to_owned(),
                starts => format!(
                    "Document functions of this name have owners {starts:?}, not {}",
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

type Key = (String, u32);
type Derived = BTreeMap<Key, Result<HirFnFacts, String>>;

/// A captured function's key: its name and its body's compiler owner.
fn key(facts: &HirFnFacts) -> Key {
    (facts.name.clone(), facts.body.hir.owner)
}

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

fn unread(channel: &str, reason: &str) -> Difference {
    Difference {
        channel: channel.into(),
        path: String::new(),
        reference: "present".into(),
        derived: reason.into(),
    }
}
