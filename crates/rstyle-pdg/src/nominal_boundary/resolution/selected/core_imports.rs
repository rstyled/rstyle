//! Exact pinned core bindings; type names do not themselves certify derive names.
use super::*;

pub(super) const CAPABILITY: &str = "rust.concrete-option-propagation";
const FMT_SOURCE: &str = "sha256:ab918e40543739be20757973f0d4abefcd9624d3759a64fc4147f1d8fc4bc035";

pub(super) fn debug(
    facts: &HirFnFacts,
    environment: &Environment,
    binding: (&[String], &str, &[String]),
) -> Result<bool, String> {
    let (scope, name, target) = binding;
    if !supports(facts, CAPABILITY)? || name != "Debug" || target != ["core", "fmt", "Debug"] {
        return Ok(false);
    }
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Core Debug import requires its pinned compiler/target")?;
    pinned_debug(registry)?;

    // These exact bytes declare the trait and separately the rustc builtin
    // derive, then reexport both. This proves neither arbitrary type imports
    // nor arbitrary derive aliases safe.
    imports::library_root(facts, environment, scope, "core")?;
    Ok(true)
}

fn pinned_debug(registry: &crate::data_library::Registry) -> Result<(), String> {
    if registry
        .cleanup_macro_source_digests
        .get("core/src/fmt/mod.rs")
        .map(String::as_str)
        != Some(FMT_SOURCE)
    {
        return Err("Core Debug trait/derive source authority is incomplete".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_import_needs_the_exact_reviewed_trait_and_builtin_derive_source() {
        let mut registry: crate::data_library::Registry = serde_json::from_str(include_str!(
            "../../../../contracts/read-only-lengths-v2.json"
        ))
        .unwrap();
        assert!(pinned_debug(&registry).is_ok());
        registry
            .cleanup_macro_source_digests
            .insert("core/src/fmt/mod.rs".into(), "sha256:changed".into());
        assert!(pinned_debug(&registry).is_err());
        registry.cleanup_macro_source_digests.clear();
        assert!(pinned_debug(&registry).is_err());
    }
}
