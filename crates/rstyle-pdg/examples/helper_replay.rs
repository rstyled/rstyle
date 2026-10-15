//! Replay the ordinary-library helper stage from an explicit diagnostic snapshot.
use rstyle_pdg::{HirFnFacts, diagnostics::Stage, unit_store::UnitStore};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    version: u32,
    scope: String,
    phase: String,
    facts: Vec<HirFnFacts>,
}

fn load(path: &Path) -> Result<Snapshot, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let snapshot: Snapshot = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if snapshot.version != 1
        || snapshot.scope != "diagnostic-raw-facts-only"
        || snapshot.phase != "pre-helper-certification"
    {
        return Err("expected version 1 pre-helper-certification diagnostic facts".into());
    }

    Ok(snapshot)
}

fn replay(path: &Path, generated: bool, storage: bool) -> Result<(), String> {
    let snapshot =
        Stage::new("replay.load", &path.display().to_string(), None).observe(|| load(path))?;
    let capture = || UnitStore::capture(snapshot.facts);
    let store = if generated {
        rstyle_pdg::verification_policy::generated(capture)?
    } else {
        capture()?
    };
    if storage {
        let measure = || storage_report(&store);
        let report = if generated {
            rstyle_pdg::verification_policy::generated(measure)?
        } else {
            measure()?
        };
        println!("{report}");
    }
    println!(
        "{}",
        serde_json::json!({
            "scope": "diagnostic-helper-certification",
            "boundary": "pre-helper-certification-facts",
        "verification_policy": if generated { "trusted-generated" } else { "strict" },
            "functions": store.functions().len(),
            "unique_helpers": store.helper_count(),
            "status": "stage-completed",
            "source_to_pair_complete": false,
        })
    );

    Ok(())
}

fn storage_report(store: &UnitStore) -> Result<serde_json::Value, String> {
    let mut bundle = Vec::new();
    store.write_bundle(&mut bundle)?;
    Ok(serde_json::json!({
        "scope":"shared-storage", "shared_bundle_bytes":bundle.len(),
        "unique_graphs":store.graph_count(), "unique_helpers":store.helper_count(),
    }))
}

fn main() -> std::process::ExitCode {
    let Some(path) = std::env::args_os().nth(1) else {
        eprintln!(
            "usage: helper_replay <pre-helper-certification-snapshot.json> [--generated] [--storage]"
        );
        return std::process::ExitCode::from(2);
    };
    let generated = std::env::args_os()
        .nth(2)
        .is_some_and(|flag| flag == "--generated");
    let storage = std::env::args_os().any(|flag| flag == "--storage");
    match replay(Path::new(&path), generated, storage) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("helper replay incomplete: {error}");
            std::process::ExitCode::from(2)
        }
    }
}
