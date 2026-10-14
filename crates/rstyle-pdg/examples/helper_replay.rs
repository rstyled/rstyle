//! Replay the ordinary-library helper stage from an explicit diagnostic snapshot.
use rstyle_pdg::{HirFnFacts, diagnostics::Stage, helper_calls};
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

fn replay(path: &Path, generated: bool) -> Result<(), String> {
    let mut snapshot =
        Stage::new("replay.load", &path.display().to_string(), None).observe(|| load(path))?;
    if generated {
        rstyle_pdg::verification_policy::generated(|| helper_calls::capture(&mut snapshot.facts))?;
    } else {
        helper_calls::capture(&mut snapshot.facts)?;
    }
    println!(
        "{}",
        serde_json::json!({
            "scope": "diagnostic-helper-certification",
            "boundary": "pre-helper-certification-facts",
        "verification_policy": if generated { "trusted-generated" } else { "strict" },
            "functions": snapshot.facts.len(),
            "status": "stage-completed",
            "source_to_pair_complete": false,
        })
    );

    Ok(())
}

fn main() -> std::process::ExitCode {
    let Some(path) = std::env::args_os().nth(1) else {
        eprintln!("usage: helper_replay <pre-helper-certification-snapshot.json> [--generated]");
        return std::process::ExitCode::from(2);
    };
    let generated = std::env::args_os()
        .nth(2)
        .is_some_and(|flag| flag == "--generated");
    match replay(Path::new(&path), generated) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("helper replay incomplete: {error}");
            std::process::ExitCode::from(2)
        }
    }
}
