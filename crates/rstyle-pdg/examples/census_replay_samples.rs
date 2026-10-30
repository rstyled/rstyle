//! Prebuild, then measure raw observation decoding without compilation or proofs.
use rstyle_pdg::{HirFnFacts, nominal_boundary::census::Census, unit_store};
use std::{collections::BTreeSet, path::Path, sync::Arc, time::Instant};

fn census_owners(facts: &[HirFnFacts]) -> usize {
    facts
        .iter()
        .filter_map(|function| {
            function
                .build
                .frontend_options
                .data::<Arc<Census>>()
                .ok()
                .map(|census| Arc::as_ptr(&census) as usize)
        })
        .collect::<BTreeSet<_>>()
        .len()
}

fn sample(bytes: &[u8]) -> Result<serde_json::Value, String> {
    let started = Instant::now();
    let facts = unit_store::read_diagnostic_snapshot(bytes)?;
    let seconds = started.elapsed().as_secs_f64();
    Ok(serde_json::json!({
        "seconds": seconds,
        "functions": facts.len(),
        "census_owners": census_owners(&facts),
    }))
}

fn replay(path: &Path) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let samples = (0..3)
        .map(|_| sample(&bytes))
        .collect::<Result<Vec<_>, _>>()?;
    println!(
        "{}",
        serde_json::json!({"bundle_bytes":bytes.len(),"samples":samples})
    );
    Ok(())
}

fn main() -> Result<(), String> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: census_replay_samples <raw-unit.evidence.json>")?;
    replay(Path::new(&path))
}
