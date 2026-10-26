//! Batch source-only diagnostics share one lexical environment and captured unit.
use super::*;
use crate::{declaration_origins::Origins, generated_sources::declarations};

/// Check generated declaration source from an ordinary caller. No ownership,
/// cleanup, derive, constructor-effect or callable permission is granted.
pub fn probe_generated_declaration_sources(
    facts: &dyn crate::unit_store::EvidenceFacts,
    trees: &[&TypeTree],
) -> Result<usize, String> {
    let environment = standalone_environment(facts.facts())?;
    let origins = facts.facts().build.frontend_options.data::<Origins>()?;
    let capture = facts
        .generated_source_capture()?
        .ok_or("Missing generated source capture")?;

    for tree in trees {
        if !result_maps::validate_generated(facts.facts(), &origins, &environment, tree)? {
            return Err("Declaration probe requires a complete generated type".into());
        }
        let origin = result_maps::origin(&origins, tree)?;
        declarations::verify_in(
            declarations::IndependentInput {
                facts: facts.facts(),
                capture: &capture,
                tree,
                origin,
            },
            &environment,
        )?;
    }

    Ok(trees.len())
}
