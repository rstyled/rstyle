//! Independent evidence checks; serialized CALL checks are implemented separately.

use super::*;
use crate::{
    HirFnFacts,
    concrete_types::{Observation, TypeTree},
    extraction_profile::ExtractionProfile,
};

pub fn verify_evidence(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<Libraries, String> {
    if !ExtractionProfile::from_facts(facts)?.is_data() {
        return if facts.build.frontend_options.contains_key(KEY) {
            Err("Library resolution evidence is restricted to Data".into())
        } else {
            Ok(Libraries {
                identity: IDENTITY.into(),
                libraries: Vec::new(),
            })
        };
    }
    let libraries: Libraries = crate::data_frontend::evidence(facts, KEY)?;
    validate_libraries(&libraries)?;
    let registered = registered_build_with(facts, &libraries)?;

    let mut methods = Vec::new();
    crate::frontend::collect_kind(&facts.body, "expr_method_call", &mut methods);
    let capture = crate::data_receivers::verify_capture(facts)?;
    for node in methods.into_iter().filter(|node| operation(node).is_some()) {
        let callee = callee(node)?;
        if callee.crate_name != "core" || callee.path != operation(node).expect("filtered").path {
            return Err("Library resolution disagrees with resolved dispatch".into());
        }
        if !libraries
            .libraries
            .iter()
            .any(|lib| lib.crate_name == callee.crate_name && lib.crate_hash == callee.crate_hash)
        {
            return Err("Missing compiler-resolved core identity".into());
        }
        if registered {
            verify_signature(node, &capture)?;
        }
    }
    Ok(libraries)
}

fn validate_libraries(libraries: &Libraries) -> Result<(), String> {
    if libraries.identity != IDENTITY || libraries.libraries.len() > 1 {
        return Err("Unregistered or excessive library resolution inventory".into());
    }
    for library in &libraries.libraries {
        if library.crate_name != "core" || library.crate_hash.is_empty() {
            return Err("Incomplete or unsupported library resolution inventory".into());
        }
    }
    Ok(())
}

pub(super) fn registered_build_with(
    facts: &HirFnFacts,
    libraries: &Libraries,
) -> Result<bool, String> {
    let Some(registry) = registry_for(facts) else {
        return Ok(false);
    };
    let Some(library) = libraries.libraries.first() else {
        return Ok(false);
    };
    Ok(library.crate_hash == registry.core_hash)
}

fn verify_signature(
    node: &HirNode,
    capture: &crate::data_receivers::Capture,
) -> Result<(), String> {
    let op = operation(node).ok_or("Missing registry operation")?;
    let definition = callee(node)?;
    if definition.definition_hash != op.definition_hash {
        return Err("Registered core definition fingerprint disagrees".into());
    }

    let record = capture
        .methods
        .iter()
        .find(|method| method.call == node.hir)
        .ok_or("Missing registered method receiver")?;
    let result = Observation::Captured {
        ty: TypeTree::Scalar(op.result.clone()),
    };
    if record.signature_inputs != [record.adjusted_type.clone()]
        || record.signature_result != result
    {
        return Err("Registered length signature disagrees with instantiated evidence".into());
    }
    Ok(())
}
