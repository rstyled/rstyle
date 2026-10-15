//! Source-aware bundle adapter; ordinary frontend JSON remains unchanged.
use super::*;
use crate::source_table;
mod records;
use records::SourceRecord;

pub(crate) struct Export<'a>(Vec<(&'a str, EntryView<'a>)>);

enum EntryView<'a> {
    Ordinary(&'a Entry),
    Source(KnownExport<'a>),
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum KnownExport<'a> {
    Inline(&'a Entry),
    Census(records::CensusExport<'a>),
    Origins(records::OriginsExport<'a>),
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum KnownImport {
    Inline(Value),
    Census(records::CensusImport),
    Origins(records::OriginsImport),
}

#[derive(Deserialize)]
#[serde(transparent)]
pub(crate) struct Import(BTreeMap<String, Value>);

impl<'a> Export<'a> {
    pub fn new(
        options: &'a FrontendOptions,
        sources: &mut source_table::Owners<'a>,
    ) -> Result<Self, String> {
        options
            .0
            .iter()
            .map(|(key, entry)| {
                let value = match entry {
                    Entry::Census(record) => EntryView::Source(KnownExport::Census(
                        records::CensusExport::new(&record.value, sources)?,
                    )),
                    Entry::Origins(record) => EntryView::Source(KnownExport::Origins(
                        records::OriginsExport::new(record, sources)?,
                    )),
                    _ if source_key(key) => EntryView::Source(KnownExport::Inline(entry)),
                    _ => EntryView::Ordinary(entry),
                };
                Ok((key.as_str(), value))
            })
            .collect::<Result<_, String>>()
            .map(Self)
    }
}

impl Import {
    pub fn decode(
        self,
        sources: &mut source_table::Imported<'_>,
    ) -> Result<FrontendOptions, String> {
        let mut entries = BTreeMap::new();
        for (key, value) in self.0 {
            let entry = if source_key(&key) {
                let encoded: KnownImport =
                    serde_json::from_value(value).map_err(|error| error.to_string())?;
                encoded.decode(&key, sources)?
            } else {
                Entry::new(&key, value)
            };
            entries.insert(key, entry);
        }
        Ok(FrontendOptions(entries))
    }
}

impl KnownImport {
    fn decode(self, key: &str, sources: &mut source_table::Imported<'_>) -> Result<Entry, String> {
        match self {
            Self::Inline(value) => Ok(Entry::new(key, value)),
            Self::Census(value) if key == crate::nominal_boundary::census::KEY => {
                value.decode(sources)
            }
            Self::Origins(value) if key == crate::declaration_origins::KEY => value.decode(sources),
            _ => Err("Wrong-kind source evidence reference".into()),
        }
    }
}

fn source_key(key: &str) -> bool {
    matches!(
        key,
        crate::nominal_boundary::census::KEY | crate::declaration_origins::KEY
    )
}

impl Serialize for EntryView<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Ordinary(value) => value.serialize(serializer),
            Self::Source(value) => value.serialize(serializer),
        }
    }
}

impl Serialize for Export<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(self.0.iter().map(|(key, entry)| (key, entry)))
    }
}

#[cfg(test)]
mod tests;
