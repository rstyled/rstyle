//! Exact immutable unit inventories; source association is checked on every use.
use super::*;
use crate::inherent_calls::{Association, UNIT_KEY, UnitDefinitions};

type Index = BTreeMap<String, usize>;

#[derive(Clone, Debug)]
pub(super) struct Inventory {
    pub(super) record: Record<Arc<UnitDefinitions>>,
    checked: Arc<OnceLock<Result<Index, String>>>,
}

impl Inventory {
    pub(super) fn new(record: Record<Arc<UnitDefinitions>>) -> Self {
        Self {
            record,
            checked: Arc::new(OnceLock::new()),
        }
    }

    fn index(&self) -> Result<&Index, String> {
        self.checked
            .get_or_init(|| {
                self.record.value.validate()?;
                Ok(self
                    .record
                    .value
                    .definitions
                    .iter()
                    .enumerate()
                    .map(|(index, item)| (item.definition_identity.clone(), index))
                    .collect())
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        self.index().map(|_| ())
    }

    fn association(&self, identity: &str) -> Result<&Association, String> {
        let index = self
            .index()?
            .get(identity)
            .ok_or("Missing inherent source-definition association")?;
        Ok(&self.record.value.definitions[*index])
    }
}

impl FrontendOptions {
    /// Share one exact typed inventory and its immutable validation state.
    pub fn share_inherent_inventory(&mut self, owner: &Self) -> Result<(), String> {
        let Some(entry @ Entry::InherentUnits(_)) = owner.0.get(UNIT_KEY) else {
            return Err("Missing typed inherent unit inventory owner".into());
        };
        self.0.insert(UNIT_KEY.into(), entry.clone());
        Ok(())
    }

    pub(crate) fn inherent_association(
        &self,
        identity: &str,
    ) -> Result<Cow<'_, Association>, String> {
        if let Some(Entry::InherentUnits(inventory)) = self.0.get(UNIT_KEY) {
            return inventory.association(identity).map(Cow::Borrowed);
        }
        // JSON mutation adapters retain the original full validation behavior.
        let value = self.decode::<Arc<UnitDefinitions>>(UNIT_KEY)?;
        value.validate()?;
        value
            .definitions
            .iter()
            .find(|item| item.definition_identity == identity)
            .cloned()
            .map(Cow::Owned)
            .ok_or_else(|| "Missing inherent source-definition association".into())
    }
}

impl DataEvidence for Arc<UnitDefinitions> {
    const KEY: &'static str = UNIT_KEY;
    fn borrow(options: &FrontendOptions) -> Option<&Self> {
        match options.0.get(Self::KEY) {
            Some(Entry::InherentUnits(record)) => Some(&record.record.value),
            _ => None,
        }
    }
    fn install(self, options: &mut FrontendOptions) {
        options.0.insert(
            Self::KEY.into(),
            Entry::InherentUnits(Inventory::new(Record::new(self, Vec::new()))),
        );
    }
}

#[cfg(test)]
mod tests;
