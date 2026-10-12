//! R0 normal-flow producer over compiler-resolved frontend evidence.
//! Eligibility, lowering, graph analysis, serialization, and admission remain
//! separate stages; a successful compiler run is not a conformance claim.

mod analysis;
mod casts;
mod emit;
mod graph;
mod hazards;
mod lower;
mod profile;
mod summaries;
mod verify;
pub use verify::verify;

/// Build and validate one bounded normal-flow pair from captured frontend facts.
pub fn produce(facts: &crate::HirFnFacts) -> Result<Pair, Vec<String>> {
    let graph = lower(facts).map_err(|e| {
        e.diagnostics
            .into_iter()
            .map(|d| d.reason)
            .collect::<Vec<_>>()
    })?;
    let analysis = analyze_facts(&graph, facts).map_err(|e| vec![e])?;
    let revision = facts
        .build
        .producer_revision
        .known_value()
        .ok_or_else(|| vec!["Missing captured producer revision".to_string()])?;
    let mut pair = emit(facts, &graph, &analysis, revision).map_err(|e| vec![e])?;
    verify(&pair, facts)?;
    let profile =
        crate::extraction_profile::ExtractionProfile::from_facts(facts).map_err(|e| vec![e])?;
    pair.pdg.extraction_evidence.conformance = vec![crate::Conformance {
        profile: pair.pdg.profile.clone(),
        status: crate::ConformanceStatus::Conformant,
        reasons: vec![conformance_reason(profile)],
    }];

    Ok(pair)
}

/// Keep historical R0 wording distinct from local expansion conformance.
fn conformance_reason(profile: crate::extraction_profile::ExtractionProfile) -> String {
    if profile == crate::extraction_profile::ExtractionProfile::R0 {
        return "Validated compat.rust.hir-r0/0.1.0 normal-flow extraction within the closed scalar subset exercised by the independent R0 fixtures; exceptional flow/cleanup omitted, no purity or rewrite-safety claim".to_string();
    }
    let policy = profile.policy();

    format!(
        "Validated {}/{} bounded normal-flow extraction; local expansions are not pinned compat R0 conformance; exceptional flow/cleanup omitted, no purity or rewrite-safety claim",
        policy.id, policy.version
    )
}

/// Verify exact serialized identities before decoding and checking the pair.
pub fn verify_serialized(
    serialized: &SerializedPair,
    facts: &crate::HirFnFacts,
) -> Result<Pair, Vec<String>> {
    let pdg = serde_json::from_slice(&serialized.pdg_bytes).map_err(|e| vec![e.to_string()])?;
    let cfg = serde_json::from_slice(&serialized.cfg_bytes).map_err(|e| vec![e.to_string()])?;
    let pair = Pair { pdg, cfg };
    serialized.verify_identities(&pair).map_err(|e| vec![e])?;
    verify(&pair, facts)?;

    Ok(pair)
}

pub use analysis::{Analysis, DefinitionSet, Dependence, analyze, analyze_cyclic, reachability};

fn analyze_facts(graph: &Graph, facts: &crate::HirFnFacts) -> Result<Analysis, String> {
    if crate::extraction_profile::ExtractionProfile::from_facts(facts)?.scalar_loops()
        && graph
            .nodes
            .iter()
            .any(|node| matches!(node.construct.as_deref(), Some("while" | "loop")))
    {
        analyze_cyclic(graph)
    } else {
        analyze(graph)
    }
}
pub use emit::{DocumentIdentity, Manifest, Pair, SerializedPair, emit};
pub use graph::{Alternative, Atom, Graph, Guard, Operation, Transfer, ValueSite};
pub use lower::lower;
