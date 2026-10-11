//! R0 normal-flow producer over compiler-resolved frontend evidence.
//! Eligibility, lowering, graph analysis, serialization, and admission remain
//! separate stages; a successful compiler run is not a conformance claim.

mod analysis;
mod emit;
mod graph;
mod hazards;
mod lower;
pub mod pinned;
mod verify;
pub use verify::verify;

/// Build and validate one bounded normal-flow pair from captured frontend facts.
pub fn produce(facts: &crate::R0FnFacts) -> Result<Pair, Vec<String>> {
    let graph = lower(facts).map_err(|e| {
        e.diagnostics
            .into_iter()
            .map(|d| d.reason)
            .collect::<Vec<_>>()
    })?;
    let analysis = analyze(&graph).map_err(|e| vec![e])?;
    let revision = facts
        .build
        .producer_revision
        .known_value()
        .ok_or_else(|| vec!["Missing captured producer revision".to_string()])?;
    let mut pair = emit(facts, &graph, &analysis, revision).map_err(|e| vec![e])?;
    verify(&pair, facts)?;
    pair.pdg.extraction_evidence.conformance = crate::Conformance {
        status: crate::ConformanceStatus::Conformant,
        reasons: vec!["Validated compat.rust.hir-r0/0.1.0 normal-flow extraction within the closed scalar subset exercised by the independent R0 fixtures; exceptional flow/cleanup omitted, no purity or rewrite-safety claim".to_string()],
    };

    Ok(pair)
}

/// Verify exact serialized identities before decoding and checking the pair.
pub fn verify_serialized(
    serialized: &SerializedPair,
    facts: &crate::R0FnFacts,
) -> Result<Pair, Vec<String>> {
    let pdg = pinned::read_pdg(&serialized.pdg_bytes).map_err(|e| vec![e])?;
    let cfg = serde_json::from_slice(&serialized.cfg_bytes).map_err(|e| vec![e.to_string()])?;
    let pair = Pair { pdg, cfg };
    serialized.verify_identities(&pair).map_err(|e| vec![e])?;
    verify(&pair, facts)?;

    Ok(pair)
}

pub use analysis::{Analysis, DefinitionSet, Dependence, analyze, reachability};
pub use emit::{DocumentIdentity, Manifest, Pair, SerializedPair, emit};
pub use graph::{Alternative, Atom, Graph, Guard, Operation, Transfer, ValueSite};
pub use lower::lower;
