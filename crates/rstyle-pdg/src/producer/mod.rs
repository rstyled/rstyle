//! R0 normal-flow producer over compiler-resolved frontend evidence.
//! Eligibility, lowering, graph analysis, serialization, and admission remain
//! separate stages; a successful compiler run is not a conformance claim.

mod analysis;
mod casts;
mod emit;
mod graph;
mod hazards;
mod lower;
pub mod matching_build;
pub mod memory;
mod profile;
mod summaries;
mod unsigned_ranges;
pub(crate) use unsigned_ranges::{bounded_value_index, verify_range_policy};
mod verify;
pub use verify::{verify, verify_view};

/// Build and validate one bounded normal-flow pair from captured frontend facts.
pub fn produce(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<Pair, Vec<String>> {
    crate::verification_session::within(|| produce_function(facts))
}

fn produce_function(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<Pair, Vec<String>> {
    let observation = crate::diagnostics::Stage::new("producer.produce", &facts.name, None).enter();
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

    observation.finish(Ok(pair))
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
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Pair, Vec<String>> {
    verify_serialized_view(serialized.as_view(), facts)
}

/// Validate a borrowed byte snapshot without copying documents or the manifest.
/// Strict/generated verification policy is identical to `verify_serialized`.
pub fn verify_serialized_view(
    serialized: SerializedPairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Pair, Vec<String>> {
    let pdg = serde_json::from_slice::<crate::transport::Document>(serialized.pdg_bytes)
        .map_err(|e| vec![e.to_string()])?
        .0;
    let cfg = serde_json::from_slice(serialized.cfg_bytes).map_err(|e| vec![e.to_string()])?;
    let pair = Pair { pdg, cfg };
    serialized.verify_identities(&pair).map_err(|e| vec![e])?;
    verify(&pair, facts)?;

    Ok(pair)
}

pub use analysis::{Analysis, DefinitionSet, Dependence, analyze, analyze_cyclic, reachability};

fn analyze_facts(
    graph: &Graph,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Analysis, String> {
    if crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .supports("rust.scalar-loops")
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
pub use emit::{
    DocumentIdentity, Manifest, Pair, PairView, SerializedPair, SerializedPairView, emit,
};
pub use graph::{Alternative, Atom, Graph, Guard, Operation, Transfer, ValueSite};
pub use lower::lower;
