//! Source-requested generated tuple cleanup, before nominal permission selection.
use super::*;
use crate::{
    generated_sources::declarations::CheckedGeneratedDeclaration, result_maps::TransportRequest,
};

/// A source request is necessary; a later semantic map witness consumes this proof.
/// This never asks nominal resolution or helper linking to initialize recursively.
pub(in crate::nominal_boundary::resolution) fn verify(
    facts: &HirFnFacts,
    resolution: &Resolution,
    request: &TransportRequest,
    declaration: &CheckedGeneratedDeclaration,
) -> Result<CheckedNoDrop, String> {
    resolution.environment.check_context(facts)?;
    let [payload, error] = request.cleanup_requests();
    if declaration.tree() != error || declaration.constructor() != request.constructors()[1] {
        return Err(
            "Generated tuple cleanup request belongs to another exact declaration or constructor"
                .into(),
        );
    }
    concrete_types::validate(error)?;
    let (nominal, _, _, fields) = error
        .generated_parts()
        .ok_or("Generated tuple cleanup requires its explicit observed declaration")?;
    let [field] = fields else {
        return Err("Generated tuple cleanup requires exactly one payload".into());
    };
    if &field.ty != payload || !scalar_errors::supported(payload) {
        return Err(
            "Generated tuple cleanup requires its exact original named scalar error payload".into(),
        );
    }

    super::verify(facts, payload, &resolution.environment)?;
    let mut target = declaration.namespace().to_vec();
    target.push(declaration.item().ident.to_string());
    if target.join("::") != nominal.definition_path {
        return Err(
            "Generated tuple cleanup source namespace differs from its exact nominal".into(),
        );
    }
    super::verify_source(facts, &resolution.environment, &target, true)?;
    Ok(CheckedNoDrop {
        nominal: nominal.clone(),
    })
}
