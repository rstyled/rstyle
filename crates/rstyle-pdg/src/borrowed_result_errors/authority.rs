//! Reviewed core method body; the unused eager error is dropped under Some.
use super::*;

const OPTION_SOURCE: &str =
    "sha256:693df7245cf3e669b2c442b62c96c76c43ad1f9cfe79f6bf4306e71c32fa9f5d";

pub(super) fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing eager ok_or core resolution")?;
    if resolution.kind != "resolved_callee" || resolution.target.is_some() {
        return Err("Eager ok_or requires a direct core definition".into());
    }
    let callee = crate::data_library::callee(node)?;
    crate::data_library::verify_core_definition(
        &callee,
        facts,
        crate::data_library::CoreDefinition {
            paths: &[METHOD, CORE_METHOD],
            unavailable: "Unpinned eager ok_or compiler/target",
            mismatch: "Eager ok_or lacks its exact reviewed core definition",
        },
    )?;
    crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "Eager ok_or")?;
    let registry =
        crate::data_library::registry_for(facts).ok_or("Missing eager ok_or registry")?;
    pinned_source(registry)?;
    if node
        .dispatch
        .as_ref()
        .is_none_or(|dispatch| dispatch.op != callee.path)
    {
        return Err("Eager ok_or dispatch and resolved definition disagree".into());
    }

    Ok(())
}

pub(super) fn pinned_source(registry: &crate::data_library::Registry) -> Result<(), String> {
    if registry
        .result_propagation_source_digests
        .get("option.rs")
        .map(String::as_str)
        != Some(OPTION_SOURCE)
    {
        return Err("Eager ok_or requires its complete reviewed Option source".into());
    }

    Ok(())
}
