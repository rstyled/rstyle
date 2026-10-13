//! Independent evidence checks; serialized CALL checks are implemented separately.

use super::*;
use crate::{
    HirFnFacts,
    concrete_types::{Observation, TypeTree},
    extraction_profile::ExtractionProfile,
};

pub fn verify_evidence(facts: &HirFnFacts) -> Result<Artifacts, String> {
    if ExtractionProfile::from_facts(facts)? != ExtractionProfile::DataV1 {
        return if facts.build.frontend_options.contains_key(KEY) {
            Err("Library artifact evidence is restricted to Data".into())
        } else {
            Ok(Artifacts {
                identity: IDENTITY.into(),
                libraries: Vec::new(),
            })
        };
    }
    let artifacts: Artifacts = crate::data_frontend::evidence(facts, KEY)?;
    validate_artifacts(&artifacts)?;
    let registered = registered_build_with(facts, &artifacts)?;

    let mut methods = Vec::new();
    crate::frontend::collect_kind(&facts.body, "expr_method_call", &mut methods);
    let capture = crate::data_receivers::verify(facts)?;
    for node in methods.into_iter().filter(|node| operation(node).is_some()) {
        let callee = callee(node)?;
        if callee.crate_name != "core" || callee.path != operation(node).expect("filtered").path {
            return Err("Library resolution disagrees with resolved dispatch".into());
        }
        if !artifacts
            .libraries
            .iter()
            .any(|lib| lib.crate_name == callee.crate_name && lib.crate_hash == callee.crate_hash)
        {
            return Err("Missing exact resolved core artifact evidence".into());
        }
        if registered {
            verify_signature(node, &capture)?;
        }
    }
    Ok(artifacts)
}

fn validate_artifacts(artifacts: &Artifacts) -> Result<(), String> {
    if artifacts.identity != IDENTITY || artifacts.libraries.len() > 1 {
        return Err("Unregistered or excessive library artifact inventory".into());
    }
    for library in &artifacts.libraries {
        if library.crate_name != "core"
            || library.crate_hash.is_empty()
            || library.files.is_empty()
            || library.files.len() > 4
        {
            return Err("Incomplete or unsupported library artifact inventory".into());
        }
        let mut formats = std::collections::BTreeSet::new();
        for file in &library.files {
            if !formats.insert(&file.format)
                || !matches!(
                    file.format.as_str(),
                    "rmeta" | "rlib" | "dylib" | "sdylib_interface"
                )
                || file.path.is_empty()
                || file.bytes == 0
                || file.bytes > MAX_FILE_BYTES
                || !file.sha256.strip_prefix("sha256:").is_some_and(|hash| {
                    hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
                })
            {
                return Err("Invalid library artifact format, identity or byte budget".into());
            }
        }
    }
    Ok(())
}

pub(super) fn registered_build_with(
    facts: &HirFnFacts,
    artifacts: &Artifacts,
) -> Result<bool, String> {
    let Some(registry) = registry_for(facts) else {
        return Ok(false);
    };
    let Some(library) = artifacts.libraries.first() else {
        return Ok(false);
    };
    if library.crate_hash != registry.core_hash {
        return Ok(false);
    }

    if library
        .files
        .iter()
        .any(|file| registry.artifacts.get(&file.format) != Some(&file.sha256))
    {
        return Err("Registered core build has mismatched artifact digests".into());
    }
    Ok(true)
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
