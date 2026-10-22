//! Transport-local references to immutable source owners; no verification token.
use crate::{ByteSpan, SourceText};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(crate) struct SourceId(pub(crate) u32);

#[derive(Default)]
pub(crate) struct Owners<'a> {
    pub snapshots: Vec<&'a str>,
    ids: BTreeMap<usize, SourceId>,
}

impl<'a> Owners<'a> {
    pub fn snapshot(&mut self, bytes: &'a crate::SourceSnapshot) -> Result<SourceId, String> {
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
    snapshots: &'a [crate::SourceSnapshot],
    used: Vec<bool>,
}

impl<'a> Imported<'a> {
    pub fn new(snapshots: &'a [crate::SourceSnapshot]) -> Self {
        Self {
            snapshots,
            used: vec![false; snapshots.len()],
        }
    }

    pub fn snapshot(&mut self, id: SourceId) -> Result<crate::SourceSnapshot, String> {
        let bytes = self
            .snapshots
            .get(id.0 as usize)
            .ok_or("Dangling bundle source reference")?;
        self.used[id.0 as usize] = true;
        Ok(bytes.clone())
    }

    pub fn range(&mut self, id: SourceId, range: ByteSpan) -> Result<SourceText, String> {
        SourceText::from_snapshot(self.snapshot(id)?, range)
    }

    pub fn finish(self) -> Result<(), String> {
        all_referenced(&self.used, "Unreferenced bundle source snapshot")
    }
}

/// Checked range owners local to one immutable normalization operation.
#[derive(Default)]
pub(crate) struct Ranges {
    records: BTreeMap<(usize, u32, u32), SourceText>,
}

impl Ranges {
    pub fn bind(
        &mut self,
        text: &SourceText,
        snapshot: &crate::SourceSnapshot,
        span: ByteSpan,
    ) -> Result<SourceText, String> {
        let key = (snapshot.as_ptr() as usize, span.start, span.end);
        if let Some(owned) = self.records.get(&key) {
            if owned != text {
                return Err("Contradictory text for one owned source range".into());
            }
            return Ok(owned.clone());
        }

        let owned = text.in_snapshot(snapshot.clone(), span)?;
        self.records.insert(key, owned.clone());
        Ok(owned)
    }
}

/// Require every record in a transport table to have an explicit consumer.
pub(crate) fn all_referenced(used: &[bool], message: &str) -> Result<(), String> {
    if used.contains(&false) {
        return Err(message.into());
    }
    Ok(())
}
