//! Strictly import a captured bundle; this does not run matching or recapture source.
use rstyle_pdg::{diagnostics::Stage, unit_store::UnitStore};
use std::path::Path;

fn replay(path: &Path) -> Result<(), String> {
    let bytes = Stage::new("replay.read", "bundle", None)
        .observe(|| std::fs::read(path).map_err(|error| error.to_string()))?;
    let store = Stage::new("replay.strict_import", "bundle", None)
        .observe(|| UnitStore::from_bundle(&bytes))?;
    println!(
        "{}",
        serde_json::json!({
            "scope": "diagnostic-strict-unit-import",
            "identity": store.identity(),
            "functions": store.functions().len(),
            "unique_helpers": store.helper_count(),
            "unique_graphs": store.graph_count(),
            "bundle_bytes": bytes.len(),
            "source_to_pair_complete": false,
            "status": "stage-completed",
        })
    );
    Ok(())
}

fn main() -> std::process::ExitCode {
    let Some(path) = std::env::args_os().nth(1) else {
        eprintln!("usage: unit_import_replay <unit.evidence.json>");
        return std::process::ExitCode::from(2);
    };
    match replay(Path::new(&path)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("strict unit import incomplete: {error}");
            std::process::ExitCode::from(2)
        }
    }
}
