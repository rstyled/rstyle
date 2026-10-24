//! Join opaque compiler selections to independently checked source and unit ownership.
use super::*;
use crate::{
    HirFnFacts,
    data_library::{self, Callee},
    nominal_constants::{CoreConstant, ProductConstant, SourceSite},
};
use serde::Deserialize;

pub(super) struct Context<'a> {
    facts: &'a dyn EvidenceFacts,
    record: ProductConstant,
    selected: SelectedUse,
}

impl<'a> Context<'a> {
    pub(super) fn new(node: &HirNode, facts: &'a dyn EvidenceFacts) -> Result<Self, String> {
        let record = crate::nominal_constants::decode(
            node.res
                .as_ref()
                .ok_or("Missing product constant resolution")?,
        )?;
        let value = facts
            .build
            .frontend_options
            .get(KEY)
            .ok_or("Missing independent product constant selections")?;
        let selections = Selections::deserialize(value).map_err(|error| error.to_string())?;
        selections.validate()?;

        let mut matches = selections
            .uses
            .into_iter()
            .filter(|selected| selected.hir == node.hir || selected.span == node.span);
        let selected = matches
            .next()
            .ok_or("Missing actual product constant use selection")?;
        if matches.next().is_some() || selected.hir != node.hir || selected.span != node.span {
            return Err("Ambiguous or split product constant use selection".into());
        }

        Ok(Self {
            facts,
            record,
            selected,
        })
    }

    fn inherent(&self, definition: &Callee, source: &SourceSite) -> Result<(), String> {
        let constructor = &self.record.constructor;
        let observed = self
            .facts
            .build
            .frontend_options
            .inherent_association(&constructor.definition.definition_hash)?;
        let nominal = self
            .record
            .ty
            .nominal_parts()
            .ok_or("Missing product nominal owner")?
            .0;
        if observed.owner_nominal != *nominal
            || observed.source_file != constructor.declaration.file
            || observed.function_span != constructor.declaration.span
            || observed.implementation_identity != constructor.implementation.definition_hash
            || observed.implementation_span != constructor.implementation_source.span
            || constructor.implementation_source.file != observed.source_file
        {
            return Err(
                "Product constructor contradicts independent inherent unit ownership".into(),
            );
        }

        let expected = if definition == &constructor.definition {
            &constructor.declaration
        } else if definition == &constructor.implementation {
            &constructor.implementation_source
        } else {
            return Err("Product definition lacks independently retained local association".into());
        };
        same_source(source, &expected.file, expected.span)
    }
}

impl crate::nominal_boundary::resolution::nominal_constants::Context for Context<'_> {
    fn build(&self, facts: &HirFnFacts) -> Result<(), String> {
        data_library::verify_64bit_sysroot(facts, "Product constant").map(|_| ())
    }

    fn local(
        &self,
        definition: &Callee,
        source: &SourceSite,
        facts: &HirFnFacts,
    ) -> Result<(), String> {
        let compiled = facts
            .build
            .frontend_options
            .get("rstyle.rust.compiled-crate")
            .ok_or("Product constant lacks actual compiled crate identity")?;
        if compiled["crate_name"].as_str() != Some(definition.crate_name.as_str())
            || definition.crate_hash != "local-unencoded"
            || definition.implementation.is_some()
        {
            return Err("Product definition differs from selected local compiled crate".into());
        }
        for observed in [
            &self.selected.constant,
            &self.selected.implementation,
            &self.selected.tuple_constructor,
        ] {
            if observed.definition == *definition {
                return same_source(source, &observed.source_file, observed.source_span);
            }
        }
        self.inherent(definition, source)
    }

    fn core_constant(&self, constant: &CoreConstant, facts: &HirFnFacts) -> Result<(), String> {
        let registry =
            data_library::registry_for(facts).ok_or("Unpinned product constant core dependency")?;
        let definition = &constant.definition;
        legacy_i32_max(constant, registry)?;

        let libraries = data_library::verify_evidence(self.facts)?;
        if !libraries.libraries.iter().any(|library| {
            library.crate_name == "core" && library.crate_hash == definition.crate_hash
        }) {
            return Err("Product constant omits its actual core dependency".into());
        }
        Ok(())
    }
}

fn legacy_i32_max(
    constant: &CoreConstant,
    registry: &data_library::Registry,
) -> Result<(), String> {
    let definition = &constant.definition;
    if !registry.exact_core_definition(definition, CORE_KEY)
        || !matches!(definition.path.as_str(), "core::i32::MAX" | "std::i32::MAX")
        || registry.source_digests.get(CORE_SOURCE).map(String::as_str) != Some(CORE_SOURCE_DIGEST)
        || constant.scalar.ty != "i32"
        || constant.scalar.value != "2147483647"
    {
        return Err("Product constant lacks reviewed legacy i32 MAX definition/source".into());
    }

    Ok(())
}

fn same_source(source: &SourceSite, file: &str, span: crate::ByteSpan) -> Result<(), String> {
    if source.file != file || source.span != span {
        return Err("Product definition source differs from independent compiler selection".into());
    }
    Ok(())
}

pub(super) const CORE_KEY: &str = "core::i32::MAX";
pub(super) const CORE_SOURCE: &str = "num/shells/legacy_int_modules.rs";
pub(super) const CORE_SOURCE_DIGEST: &str =
    "sha256:6321882b5e4b268313f5d29890c6ccbb61095bf946777e30249396bd7273a3d4";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn std_display_alias_preserves_exact_core_identity_for_each_reviewed_target() {
        for registry in data_library::registries() {
            let mut constant = CoreConstant {
                definition: Callee {
                    path: CORE_KEY.into(),
                    crate_name: "core".into(),
                    crate_hash: registry.core_hash.clone(),
                    definition_hash: registry.definitions[CORE_KEY].clone(),
                    implementation: None,
                },
                scalar: crate::nominal_constants::ScalarValue {
                    ty: "i32".into(),
                    value: "2147483647".into(),
                },
            };
            for path in ["core::i32::MAX", "std::i32::MAX"] {
                constant.definition.path = path.into();
                legacy_i32_max(&constant, registry).unwrap();
            }
            for field in [
                "path",
                "crate",
                "crate_hash",
                "definition_hash",
                "type",
                "value",
            ] {
                let mut changed = constant.clone();
                match field {
                    "path" => changed.definition.path = "user::i32::MAX".into(),
                    "crate" => changed.definition.crate_name = "std".into(),
                    "crate_hash" => changed.definition.crate_hash = "forged".into(),
                    "definition_hash" => changed.definition.definition_hash = "forged".into(),
                    "type" => changed.scalar.ty = "u32".into(),
                    "value" => changed.scalar.value = "2147483646".into(),
                    _ => unreachable!(),
                }
                assert!(legacy_i32_max(&changed, registry).is_err(), "{field}");
            }
        }
    }
}
