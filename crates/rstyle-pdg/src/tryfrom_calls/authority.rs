use super::*;

pub(crate) struct RegistryAuthority;

impl CoreAuthority for RegistryAuthority {
    fn verify(
        &self,
        facts: &HirFnFacts,
        role: CoreRole,
        definition: &Definition,
    ) -> Result<(), String> {
        let path = match role {
            CoreRole::Trait => "core::convert::TryFrom",
            CoreRole::Method => "core::convert::TryFrom::try_from",
            CoreRole::ErrorMember => "core::convert::TryFrom::Error",
            CoreRole::Result => "core::result::Result",
        };
        if definition.local
            || definition.crate_name != "core"
            || definition.path.replace("std::", "core::") != path
        {
            return Err("Concrete TryFrom authority received another core role".into());
        }
        let registry = crate::data_library::registry_for(facts)
            .ok_or("Unpinned concrete TryFrom compiler/target")?;
        crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "concrete TryFrom")?;
        let key = match role {
            CoreRole::Trait => "rust.concrete-tryfrom.trait",
            CoreRole::Method => "rust.concrete-tryfrom.method",
            CoreRole::ErrorMember => "rust.concrete-tryfrom.error",
            CoreRole::Result => "rust.concrete-tryfrom.result",
        };
        if registry.definitions.get(key) != Some(&definition.definition_identity)
            || registry.definitions.get("rust.concrete-tryfrom.core-crate")
                != Some(&definition.stable_crate_id)
        {
            return Err("Concrete TryFrom lacks exact reviewed core role authority".into());
        }

        for (file, digest) in crate::result_propagation::authority::SOURCE_DIGESTS {
            let actual = match *file {
                "result.rs" => registry.source_digests.get(*file),
                "convert/mod.rs" => registry.result_propagation_source_digests.get(*file),
                _ => continue,
            };
            if actual.map(String::as_str) != Some(*digest) {
                return Err("Concrete TryFrom lacks reviewed core conversion/Result source".into());
            }
        }
        Ok(())
    }
}
