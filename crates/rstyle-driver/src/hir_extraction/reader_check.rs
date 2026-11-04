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

pub(super) fn check(
    tcx: TyCtxt<'_>,
    (options, passes): (&super::Options<'_>, cpg_reader::Passes),
    facts: &[HirFnFacts],
) -> Result<(), String> {
    let Some(dir) = std::env::var_os(DIR_ENV) else {
        return Ok(());
    };
    // Without a source root, the crate root file's directory holds its sources.
    let root = match options.source_root {
        Some(root) => root.to_path_buf(),
        None => crate_directory(tcx).ok_or("The crate has no root source file")?,
    };
    // A failure to read the documents is the comparison's finding, never the capture's.
    let package = options.unit_context.map(|context| &context.package);
    let derived =
        derived(tcx, (&root, package), (options.selected(), passes)).unwrap_or_else(|error| {
            let mut unread = BTreeMap::new();
            for reference in facts {
                unread.insert(
                    (reference.name.clone(), reference.fn_span.start),
                    Err(error.clone()),
                );
            }
            unread
        });

    let profile = options.selected().token();
    let lines = facts.iter().map(|reference| {
        let differences = function_differences(reference, &derived);
        json!({"function": reference.name, "profile": profile, "differences": differences})
    });
    write(tcx, std::path::Path::new(&dir), lines)
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

fn crate_directory(tcx: TyCtxt<'_>) -> Option<std::path::PathBuf> {
    let file = tcx.sess.local_crate_source_file()?;
    let path = std::path::absolute(file.local_path()?).ok()?;
    let directory = path.parent()?;
    rstyle_pdg::nominal_binding::absolute_path(directory).ok()
}

type Derived = BTreeMap<(String, u32), Result<HirFnFacts, String>>;

fn derived(
    tcx: TyCtxt<'_>,
    origin: (
        &std::path::Path,
        Option<&rstyle_pdg::nominal_binding::PackageContext>,
    ),
    (profile, passes): (
        rstyle_pdg::extraction_profile::ExtractionProfile,
        cpg_reader::Passes,
    ),
) -> Result<Derived, String> {
    let mut derived = BTreeMap::new();
    for (document, _) in crate::cpg::documents(tcx, origin) {
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
