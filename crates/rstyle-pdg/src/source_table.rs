//! Transport-local references to immutable source owners; no verification token.
use crate::{ByteSpan, SourceText};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(crate) struct SourceId(pub(crate) u32);

#[derive(Default)]
pub(crate) struct Owners<'a> {
    pub snapshots: Vec<&'a str>,
    ids: BTreeMap<usize, SourceId>,
}

impl<'a> Owners<'a> {
    pub fn snapshot(&mut self, bytes: &'a Arc<str>) -> Result<SourceId, String> {
        let pointer = bytes.as_ptr() as usize;
        let next =
            SourceId(u32::try_from(self.snapshots.len()).map_err(|_| "Oversized source table")?);
        let id = *self.ids.entry(pointer).or_insert_with(|| {
            self.snapshots.push(bytes.as_ref());
            next
        });
        Ok(id)
    }
}

pub(crate) struct Imported<'a> {
    snapshots: &'a [Arc<str>],
    used: Vec<bool>,
}

impl<'a> Imported<'a> {
    pub fn new(snapshots: &'a [Arc<str>]) -> Self {
        Self {
            snapshots,
            used: vec![false; snapshots.len()],
        }
    }

    pub fn snapshot(&mut self, id: SourceId) -> Result<Arc<str>, String> {
        let bytes = self
            .snapshots
            .get(id.0 as usize)
            .ok_or("Dangling bundle source reference")?;
        self.used[id.0 as usize] = true;
        Ok(Arc::clone(bytes))
    }

    pub fn range(&mut self, id: SourceId, range: ByteSpan) -> Result<SourceText, String> {
        SourceText::from_snapshot(self.snapshot(id)?, range)
    }

    pub fn finish(self) -> Result<(), String> {
        if self.used.contains(&false) {
            return Err("Unreferenced bundle source snapshot".into());
        }
        Ok(())
    }
}
