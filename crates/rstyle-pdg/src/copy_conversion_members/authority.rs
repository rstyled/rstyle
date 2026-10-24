//! Reuse reviewed original core pins; displayed names never authorize selection.
use super::{Direction, model::Definition};
use crate::{HirFnFacts, data_library::Registry};

pub(super) struct Pins<'a> {
    pub definitions: &'a std::collections::BTreeMap<String, String>,
}

pub(super) fn reviewed(facts: &HirFnFacts) -> Result<Pins<'static>, String> {
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Unpinned Copy conversion compiler/target")?;
    crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "Copy conversion")?;
    sources(registry)?;
    Ok(Pins {
        definitions: &registry.definitions,
    })
}

fn sources(registry: &Registry) -> Result<(), String> {
    for (file, expected) in crate::result_propagation::authority::SOURCE_DIGESTS {
        let actual = match *file {
            "convert/mod.rs" => registry.result_propagation_source_digests.get(*file),
            "result.rs" => registry.source_digests.get(*file),
            _ => continue,
        };
        if actual.map(String::as_str) != Some(*expected) {
            return Err(
                "Copy conversion lacks independently reviewed core source contracts".into(),
            );
        }
    }
    Ok(())
}

impl Pins<'_> {
    pub(super) fn role(&self, definition: &Definition, role: &str) -> Result<(), String> {
        let (key, path) = match role {
            "from-trait" => ("rust.product-conversion.from-trait", "core::convert::From"),
            "from-method" => (
                "rstyle.result-propagation.identity-from.method",
                "core::convert::From::from",
            ),
            "tryfrom-trait" => ("rust.concrete-tryfrom.trait", "core::convert::TryFrom"),
            "tryfrom-method" => (
                "rust.concrete-tryfrom.method",
                "core::convert::TryFrom::try_from",
            ),
            "error" => (
                "rust.concrete-tryfrom.error",
                "core::convert::TryFrom::Error",
            ),
            "result" => ("rust.concrete-tryfrom.result", "core::result::Result"),
            _ => return Err("Unknown Copy conversion core role".into()),
        };
        if definition.local
            || definition.crate_name != "core"
            || definition.path.replace("std::", "core::") != path
            || self.definitions.get(key) != Some(&definition.definition_identity)
            || self.definitions.get("rust.concrete-tryfrom.core-crate")
                != Some(&definition.stable_crate_id)
        {
            return Err("Copy conversion differs from exact reviewed core definition".into());
        }
        Ok(())
    }

    pub(super) fn conversion(
        &self,
        direction: Direction,
        observation: &super::Observation,
    ) -> Result<(), String> {
        let (trait_role, method_role) = match direction {
            Direction::Wrap => ("from-trait", "from-method"),
            Direction::Unwrap => ("tryfrom-trait", "tryfrom-method"),
        };
        self.role(&observation.trait_definition, trait_role)?;
        self.role(&observation.trait_method, method_role)
    }
}
