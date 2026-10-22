//! Exact names imported from the reviewed pinned alloc macro namespace.
use super::{Census, expansions::FileSpan};
use crate::{data_library::Callee, unit_store::EvidenceFacts};
use serde::{Deserialize, Serialize};

pub const CAPABILITY: &str = "rust.pinned-alloc-macro-imports";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Import {
    pub declaration: FileSpan,
    pub exports: Exports,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exports {
    pub library: Callee,
    pub macros: Vec<Macro>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Macro {
    pub name: String,
    pub kind: String,
    pub public: bool,
    pub definition: Callee,
}

impl Exports {
    fn names(&self) -> Vec<String> {
        self.macros.iter().map(|item| item.name.clone()).collect()
    }

    fn matches(&self, expected: &Self) -> bool {
        self.library == expected.library
            && self.macros.len() == expected.macros.len()
            && self
                .macros
                .iter()
                .zip(&expected.macros)
                .all(|(actual, expected)| actual.matches(expected))
    }
}

impl Macro {
    fn matches(&self, expected: &Self) -> bool {
        self.name == expected.name
            && self.kind == expected.kind
            && self.public == expected.public
            && (self.definition.path == expected.definition.path
                || self.definition.path == format!("std::{}", expected.name))
            && self.definition.crate_name == expected.definition.crate_name
            && self.definition.crate_hash == expected.definition.crate_hash
            && self.definition.definition_hash == expected.definition.definition_hash
            && self.definition.implementation.is_none()
    }
}

pub(super) fn authority(facts: &dyn EvidenceFacts, census: &Census) -> Result<(), String> {
    if census.external_imports.is_empty() {
        return Ok(());
    }
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("External macro imports require their registered source capability".into());
    }
    crate::data_library::verify_sysroot_roots(facts, &["alloc", "core", "std"], "External macro")?;
    let registry = crate::data_library::registry_for(facts)
        .ok_or("External macro imports require pinned compiler/target")?;
    let expected = registry
        .alloc_macro_exports
        .as_ref()
        .ok_or("Missing reviewed complete alloc macro inventory")?;

    for import in &census.external_imports {
        if !import.exports.matches(expected) {
            return Err(
                "External macro import disagrees with the complete pinned alloc inventory".into(),
            );
        }
    }
    Ok(())
}

pub(super) fn declaration(
    item: &syn::Item,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<bool, String> {
    let syn::Item::ExternCrate(item) = item else {
        return Ok(false);
    };
    if item.ident != "alloc"
        || item.rename.is_some()
        || !matches!(item.vis, syn::Visibility::Inherited)
    {
        return Ok(false);
    }

    let mut macro_use = 0;
    for attribute in configuration.effective_attributes(&item.attrs)? {
        if matches!(&attribute, syn::Meta::Path(path) if path.is_ident("macro_use")) {
            macro_use += 1;
        } else if !["cfg", "doc", "allow", "warn", "deny", "forbid"]
            .iter()
            .any(|name| attribute.path().is_ident(name))
        {
            return Ok(false);
        }
    }
    Ok(macro_use == 1)
}

impl super::Closure<'_> {
    pub(super) fn external_import(
        &mut self,
        site: &super::Site<'_>,
        item: &syn::Item,
    ) -> Result<Option<Vec<String>>, String> {
        let Some(records) = self.external_imports else {
            return Ok(None);
        };
        let configuration = self
            .selected
            .as_ref()
            .ok_or("Missing external import configuration")?;
        if !declaration(item, configuration)? {
            return Ok(None);
        }
        let syn::Item::ExternCrate(item) = item else {
            unreachable!()
        };
        if !site.namespace.is_empty() {
            return Err("External macro import must belong to the original crate root".into());
        }

        let source = self
            .files
            .get(site.path)
            .ok_or("Missing original import source")?;
        let span = crate::ByteSpan {
            start: crate::source_evidence::original_span(source, item.extern_token.span).start,
            end: crate::source_evidence::original_span(source, item.semi_token.span).end,
        };
        let matching: Vec<_> = records
            .iter()
            .enumerate()
            .filter(|(_, record)| {
                record.declaration.path == site.path && record.declaration.span == span
            })
            .collect();
        let [(index, record)] = matching.as_slice() else {
            return Err("Missing or duplicate original external macro import observation".into());
        };
        if record.declaration.digest != crate::concrete_types::digest(source.as_bytes())
            || !self.used_external_imports.insert(*index)
        {
            return Err("External macro import source or unique ownership disagrees".into());
        }
        verify_exports(&record.exports)?;
        Ok(Some(record.exports.names()))
    }
}

fn verify_exports(exports: &Exports) -> Result<(), String> {
    crate::data_library::registries()
        .iter()
        .filter_map(|registry| registry.alloc_macro_exports.as_ref())
        .find(|expected| exports.matches(expected))
        .map(|_| ())
        .ok_or_else(|| "Unregistered or incomplete external macro namespace".into())
}

#[cfg(test)]
mod tests;
