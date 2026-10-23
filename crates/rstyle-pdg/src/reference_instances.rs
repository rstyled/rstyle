//! Independent reference-owner and conversion evidence checks.
//!
//! These checks grant no body substitution, returned-origin, cleanup, callable,
//! artifact, or admission authority. Canonical original facts remain unchanged.
mod body;
mod operations;
mod provenance;
mod view_types;

use crate::{HirFnFacts, extraction_profile::ExtractionProfile};
use serde_json::Value;

/// Verify the source-bound owner and exact conversion of an original constructor.
/// Other body operations and projected value types require their own proof.
pub fn verify_constructor_observations(
    canonical: &HirFnFacts,
    provenance: &Value,
    conversions: &[Value],
    haystack: &str,
) -> Result<(), String> {
    verify_owner(canonical, provenance, haystack)?;
    operations::verify(canonical, conversions, haystack)
}

/// Verify a forwarder's source-bound owner. This does not verify its call target,
/// the outer selected trait implementation, or the returned product summary.
pub fn verify_forwarder_owner(
    canonical: &HirFnFacts,
    provenance: &Value,
    haystack: &str,
) -> Result<(), String> {
    verify_owner(canonical, provenance, haystack)
}

/// Check the complete original constructor's saved-value flow and projected
/// observation roster. Opaque compiler type IDs remain consistency claims;
/// this grants no concrete owner, artifact, origin, summary or admission proof.
pub fn verify_constructor_body_observations(
    canonical: &HirFnFacts,
    observation: &Value,
    haystack: &str,
) -> Result<(), String> {
    verify_source(canonical)?;
    body::verify(canonical, observation, haystack)
}

fn verify_owner(canonical: &HirFnFacts, provenance: &Value, haystack: &str) -> Result<(), String> {
    verify_source(canonical)?;
    provenance::verify(canonical, provenance, haystack)
}

fn verify_source(canonical: &HirFnFacts) -> Result<(), String> {
    if ExtractionProfile::from_facts(canonical)? != ExtractionProfile::DataV3 {
        return Err("Reference observation proof requires original DataV3 facts".into());
    }
    crate::nominal_boundary::resolution::verify_source_inputs(canonical)?;
    crate::source_evidence::verify(canonical)
}
