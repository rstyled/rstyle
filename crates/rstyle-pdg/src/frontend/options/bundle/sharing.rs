//! Inline-first metadata references scoped to one numeric snapshot.
use super::*;

pub(super) fn key(key: &str) -> bool {
    matches!(key, "invocation" | crate::nominal_boundary::CONTEXT_KEY)
}

#[derive(Default)]
pub(super) struct Export<'a> {
    pub enabled: bool,
    entries: Vec<(&'a str, &'a Entry)>,
}

impl<'a> Export<'a> {
    pub fn reference(&mut self, key: &'a str, entry: &'a Entry) -> Result<Option<u32>, String> {
        let same_owner = self.entries.iter().position(|(known_key, known)| {
            *known_key == key && std::ptr::eq(known.json(), entry.json())
        });
        let existing = same_owner.or_else(|| {
            self.entries
                .iter()
                .position(|(known_key, known)| *known_key == key && known.equivalent(entry))
        });

        if let Some(index) = existing {
            return u32::try_from(index)
                .map(Some)
                .map_err(|_| "Oversized shared metadata table".into());
        }

        u32::try_from(self.entries.len()).map_err(|_| "Oversized shared metadata table")?;
        self.entries.push((key, entry));
        Ok(None)
    }
}

#[derive(Default)]
pub(super) struct Import {
    pub enabled: bool,
    entries: Vec<(String, Entry)>,
}

impl Import {
    pub fn retain(&mut self, key: &str, value: Value) -> Entry {
        let entry = Entry::new(key, value);
        self.entries.push((key.into(), entry.clone()));
        entry
    }

    pub fn entry(&self, key: &str, index: u32) -> Result<Entry, String> {
        if !self.enabled {
            return Err("Shared metadata reference requires the current style schema".into());
        }
        let (known_key, entry) = self
            .entries
            .get(index as usize)
            .ok_or("Dangling or forward shared metadata reference")?;
        if known_key != key {
            return Err("Shared metadata reference disagrees with its option key".into());
        }
        Ok(entry.clone())
    }
}

#[cfg(test)]
mod tests;
