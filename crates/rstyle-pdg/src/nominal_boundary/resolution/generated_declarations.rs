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

/// Activation is reserved for the connected ownership/value contract.
pub(super) const CAPABILITY: &str = "rust.named-error-payload-flow";

pub(super) fn capture(
    input: &dyn crate::unit_store::EvidenceFacts,
    resolution: &Resolution,
    origins: &Origins,
) -> Result<Vec<declarations::CheckedGeneratedDeclaration>, String> {
    let trees: Vec<_> = resolution
        .trees
        .values()
        .filter(|tree| tree.generated_parts().is_some())
        .collect();
    if trees.is_empty() {
        return Ok(Vec::new());
    }
    let capture = input
        .generated_source_capture()?
        .ok_or("Missing generated declaration unit capture")?;
    trees
        .into_iter()
        .map(|tree| {
            result_maps::validate_generated(input.facts(), origins, &resolution.environment, tree)?;
            declarations::verify_in(
                declarations::IndependentInput {
                    facts: input.facts(),
                    capture: &capture,
                    tree,
                    origin: result_maps::origin(origins, tree)?,
                },
                &resolution.environment,
            )
        })
        .collect()
}

pub(super) fn contains(
    declarations: &[declarations::CheckedGeneratedDeclaration],
    key: &[String],
) -> bool {
    let Some((name, namespace)) = key.split_last() else {
        return false;
    };
    declarations
        .iter()
        .any(|declaration| declaration.namespace() == namespace && declaration.item().ident == name)
}

impl Resolution {
    pub(super) fn generated_declaration(
        &self,
        tree: &TypeTree,
    ) -> Option<&declarations::CheckedGeneratedDeclaration> {
        self.generated_declarations
            .iter()
            .find(|declaration| declaration.tree() == tree)
    }
}

/// Original declaration spelling/imports only; no cleanup or admission authority.
pub fn probe_generated_error_imports(
    input: &dyn crate::unit_store::EvidenceFacts,
) -> Result<usize, String> {
    let facts = input.facts();
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let resolution = declaration_resolution_with_generated(
        input,
        &interface,
        standalone_environment(facts)?,
        true,
    )?;
    Ok(resolution.generated_declarations.len())
}

/// Independently prove the complete payload-error cleanup closure. This probe
/// publishes no ownership/value permission to a function or serialized document.
pub fn probe_named_error_cleanup(
    input: &dyn crate::unit_store::EvidenceFacts,
) -> Result<usize, String> {
    let facts = input.facts();
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let Observation::Captured { ty } = &interface.result else {
        return Err("Cleanup probe requires a complete error result".into());
    };
    let error = match ty {
        TypeTree::Result { err, .. } => err.as_ref(),
        error => error,
    };
    let resolution = declaration_resolution_with_generated(
        input,
        &interface,
        standalone_environment(facts)?,
        true,
    )?;
    selected::no_drop::payload_errors::verify(facts, &resolution, error)?;

    Ok(resolution.trees.len())
}
