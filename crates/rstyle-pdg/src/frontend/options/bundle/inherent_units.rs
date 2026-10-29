//! One exact immutable inventory per bundle table entry, with explicit references.
use super::*;

#[derive(Default)]
pub(crate) struct Inventories<'a> {
    records: Vec<&'a super::super::inherent_units::Inventory>,
    censuses: BTreeMap<usize, (u32, &'a Arc<crate::nominal_boundary::census::Census>)>,
}

impl<'a> Inventories<'a> {
    pub(super) fn census_reference(
        &mut self,
        census: &'a Arc<crate::nominal_boundary::census::Census>,
    ) -> Result<Option<u32>, String> {
        let next = u32::try_from(self.censuses.len()).map_err(|_| "Oversized census table")?;
        match self.censuses.entry(Arc::as_ptr(census) as usize) {
            std::collections::btree_map::Entry::Occupied(entry) => Ok(Some(entry.get().0)),
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert((next, census));
                Ok(None)
            }
        }
    }

    pub(super) fn reference(
        &mut self,
        record: &'a super::super::inherent_units::Inventory,
    ) -> Result<u32, String> {
        let existing = self.records.iter().position(|known| {
            (Arc::ptr_eq(&known.record.value, &record.record.value)
                || known.record.value == record.record.value)
                && known.record.nulls == record.record.nulls
        });
        let index = existing.unwrap_or_else(|| {
            self.records.push(record);
            self.records.len() - 1
        });
        u32::try_from(index).map_err(|_| "Oversized inherent unit inventory table".into())
    }
}

impl Serialize for Inventories<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.records.iter().map(|record| record.record.render()))
    }
}

pub(crate) struct ImportedInventories {
    entries: Vec<Entry>,
    used: Vec<bool>,
    legacy: bool,
    censuses: Vec<Entry>,
    shared_censuses: bool,
}

impl ImportedInventories {
    pub(crate) fn new(values: Option<Vec<Value>>, legacy: bool) -> Result<Self, String> {
        if legacy != values.is_none() {
            return Err("Shared unit version disagrees with inherent inventory table".into());
        }

        let mut entries = Vec::new();
        let mut associations = BTreeMap::new();
        for value in values.unwrap_or_default() {
            let entry = Entry::new(crate::inherent_calls::UNIT_KEY, value);
            let Entry::InherentUnits(inventory) = &entry else {
                return Err("Malformed shared inherent unit inventory".into());
            };
            inventory.validate()?;
            for association in &inventory.record.value.definitions {
                if associations
                    .insert(association.definition_identity.clone(), association.clone())
                    .is_some_and(|known| known != *association)
                {
                    return Err(
                        "Contradictory shared inherent source-definition association".into(),
                    );
                }
            }
            if entries.iter().any(|known: &Entry| known.equivalent(&entry)) {
                return Err("Duplicate shared inherent unit inventory".into());
            }
            entries.push(entry);
        }

        Ok(Self {
            used: vec![false; entries.len()],
            entries,
            legacy,
            censuses: Vec::new(),
            shared_censuses: false,
        })
    }

    pub(crate) fn share_censuses(&mut self, enabled: bool) {
        self.shared_censuses = enabled;
    }

    pub(super) fn retain_census(&mut self, entry: Entry) -> Entry {
        if self.shared_censuses {
            self.censuses.push(entry.clone());
        }
        entry
    }

    pub(super) fn census(&self, index: u32) -> Result<Entry, String> {
        if !self.shared_censuses {
            return Err("Shared census reference requires the current bundle schema".into());
        }
        self.censuses
            .get(index as usize)
            .cloned()
            .ok_or_else(|| "Dangling or forward census reference".into())
    }

    pub(super) fn legacy(&self) -> bool {
        self.legacy
    }

    pub(super) fn legacy_entry(&mut self, value: Value) -> Entry {
        let entry = Entry::new(crate::inherent_calls::UNIT_KEY, value);
        if matches!(entry, Entry::InherentUnits(_)) {
            if let Some(known) = self.entries.iter().find(|known| known.equivalent(&entry)) {
                return known.clone();
            }
            self.entries.push(entry.clone());
            self.used.push(true);
        }
        entry
    }

    pub(super) fn entry(&mut self, index: u32) -> Result<Entry, String> {
        let index = index as usize;
        let entry = self
            .entries
            .get(index)
            .ok_or("Dangling inherent unit inventory reference")?;
        self.used[index] = true;
        Ok(entry.clone())
    }

    pub(crate) fn finish(self) -> Result<(), String> {
        crate::source_table::all_referenced(&self.used, "Unreferenced inherent unit inventory")
    }
}

#[cfg(test)]
mod tests;
