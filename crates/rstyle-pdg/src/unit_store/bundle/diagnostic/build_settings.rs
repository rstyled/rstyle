//! Raw diagnostic settings sharing never carries proof or admission authority.
use super::*;
use crate::{BuildFacts, source_table};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Exporter {
    pub settings: Vec<BuildFacts<()>>,
    indices: BTreeMap<Vec<u8>, u32>,
}

#[derive(Serialize)]
pub(super) struct Reference<'a> {
    settings: u32,
    frontend_options: option_bundle::Export<'a>,
}

impl Exporter {
    pub fn reference<'a>(
        &mut self,
        facts: &'a BuildFacts,
        sources: &mut source_table::Owners<'a>,
        inventories: &mut option_bundle::Inventories<'a>,
    ) -> Result<Reference<'a>, String> {
        let settings = self.index(facts.settings())?;
        Ok(Reference {
            settings,
            frontend_options: option_bundle::Export::new(
                &facts.frontend_options,
                sources,
                inventories,
            )?,
        })
    }

    fn index(&mut self, settings: BuildFacts<()>) -> Result<u32, String> {
        if self.settings.first() == Some(&settings) {
            return Ok(0);
        }
        let key = serde_json::to_vec(&settings).map_err(|error| error.to_string())?;
        if let Some(index) = self.indices.get(&key) {
            return Ok(*index);
        }

        let index = u32::try_from(self.settings.len()).map_err(|error| error.to_string())?;
        self.indices.insert(key, index);
        self.settings.push(settings);
        Ok(index)
    }
}

#[derive(Default)]
pub(super) struct Table(Option<Vec<BuildFacts<()>>>);

impl<'de> Deserialize<'de> for Table {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::deserialize(deserializer).map(|entries| Self(Some(entries)))
    }
}

impl Table {
    pub fn present(&self) -> bool {
        self.0.is_some()
    }

    pub fn import(self) -> Result<Importer, String> {
        Importer::new(self.0.ok_or("Raw snapshot build settings are missing")?)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ImportedReference {
    settings: u32,
    frontend_options: option_bundle::Import,
}

pub(super) struct Importer {
    settings: Vec<BuildFacts<()>>,
    used: Vec<bool>,
}

impl Importer {
    fn new(settings: Vec<BuildFacts<()>>) -> Result<Self, String> {
        let mut keys = BTreeSet::new();
        for entry in &settings {
            let key = serde_json::to_vec(entry).map_err(|error| error.to_string())?;
            if !keys.insert(key) {
                return Err("Duplicate raw snapshot build settings".into());
            }
        }
        Ok(Self {
            used: vec![false; settings.len()],
            settings,
        })
    }

    pub fn restore(
        &mut self,
        reference: ImportedReference,
    ) -> Result<BuildFacts<option_bundle::Import>, String> {
        let index = reference.settings as usize;
        let settings = self
            .settings
            .get(index)
            .ok_or("Dangling raw snapshot build settings")?;
        self.used[index] = true;
        settings
            .clone()
            .try_map_options(|()| Ok(reference.frontend_options))
    }

    pub fn finish(self) -> Result<(), String> {
        if self.used.iter().any(|used| !used) {
            return Err("Unused raw snapshot build settings".into());
        }
        Ok(())
    }
}
