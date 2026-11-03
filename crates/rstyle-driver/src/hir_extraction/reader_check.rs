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

// rstyle-allow(side_effect_density): diagnostic boundary: writes the comparison it was asked for
pub(super) fn check(
    tcx: TyCtxt<'_>,
    options: &super::Options<'_>,
    facts: &[HirFnFacts],
) -> Result<(), String> {
    let Some(dir) = std::env::var_os(DIR_ENV) else {
        return Ok(());
    };
    let root = match options.source_root {
        Some(root) => root.to_path_buf(),
        None => std::env::current_dir().map_err(|error| error.to_string())?,
    };
    // A failure to read the documents is the comparison's finding, never the capture's.
    let derived = derived(tcx, &root, options.selected()).unwrap_or_else(|error| {
        let mut unread = BTreeMap::new();
        for reference in facts {
            unread.insert(
                (reference.name.clone(), reference.fn_span.start),
                Err(error.clone()),
            );
        }
        unread
    });

    let mut lines = Vec::new();
    for reference in facts {
        let key = (reference.name.clone(), reference.fn_span.start);
        let differences = match derived.get(&key) {
            Some(Ok(derived)) => cpg_reader::compare::differences(reference, derived),
            Some(Err(reason)) => vec![unread("function.read", reason)],
            None => vec![unread(
                "function.absent",
                "No document function has this name and span",
            )],
        };
        let profile = options.selected().token();
        let line =
            json!({"function": reference.name, "profile": profile, "differences": differences});
        lines.push(line.to_string());
    }

    let name = format!(
        "{}-{:016x}-{}.jsonl",
        tcx.crate_name(rustc_span::def_id::LOCAL_CRATE),
        tcx.stable_crate_id(rustc_span::def_id::LOCAL_CRATE)
            .as_u64(),
        std::process::id()
    );
    let path = std::path::Path::new(&dir).join(name);
    let mut file =
        std::fs::File::create(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    for line in lines {
        writeln!(file, "{line}").map_err(|error| error.to_string())?;
    }
    Ok(())
}

type Derived = BTreeMap<(String, u32), Result<HirFnFacts, String>>;

fn derived(
    tcx: TyCtxt<'_>,
    root: &std::path::Path,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> Result<Derived, String> {
    let mut derived = BTreeMap::new();
    for (document, _) in crate::cpg::documents(tcx, root) {
        let document = rstyle_core::cpg::layout::bundle(vec![document])?;
        for read in cpg_reader::read(&document, profile)? {
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
