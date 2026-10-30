//! Source-aware bundle adapter; ordinary frontend JSON remains unchanged.
use super::*;
use crate::source_table;
mod inherent_units;
mod records;
mod sharing;
pub(crate) use inherent_units::{ImportedInventories, Inventories};
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
    CensusReference(u32),
    SharedReference(u32),
    Origins(records::OriginsExport<'a>),
    InherentUnit(u32),
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
    Census(Box<records::CensusImport>),
    CensusReference(u32),
    SharedReference(u32),
    Origins(records::OriginsImport),
    InherentUnit(u32),
}

#[derive(Deserialize)]
#[serde(transparent)]
pub(crate) struct Import(BTreeMap<String, Value>);

impl<'a> Export<'a> {
    pub fn new(
        options: &'a FrontendOptions,
        sources: &mut source_table::Owners<'a>,
        inventories: &mut Inventories<'a>,
    ) -> Result<Self, String> {
        options
            .0
            .iter()
            .map(|(key, entry)| {
                let value = match entry {
                    _ if inventories.shared_options.enabled && sharing::key(key) => {
                        EntryView::Source(
                            match inventories.shared_options.reference(key, entry)? {
                                Some(index) => KnownExport::SharedReference(index),
                                None => KnownExport::Inline(entry),
                            },
                        )
                    }
                    Entry::Census(record) => {
                        EntryView::Source(match inventories.census_reference(&record.value)? {
                            Some(index) => KnownExport::CensusReference(index),
                            None => KnownExport::Census(records::CensusExport::new(
                                &record.value,
                                sources,
                            )?),
                        })
                    }
                    Entry::Origins(record) => EntryView::Source(KnownExport::Origins(
                        records::OriginsExport::new(record, sources)?,
                    )),
                    Entry::InherentUnits(record) if record.validate().is_ok() => {
                        EntryView::Source(KnownExport::InherentUnit(inventories.reference(record)?))
                    }
                    _ if source_key(key) || key == crate::inherent_calls::UNIT_KEY => {
                        EntryView::Source(KnownExport::Inline(entry))
                    }
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
        inventories: &mut ImportedInventories,
    ) -> Result<FrontendOptions, String> {
        let entries = self
            .0
            .into_iter()
            .map(|(key, value)| {
                let entry = decode_entry(&key, value, sources, inventories)?;
                Ok((key, entry))
            })
            .collect::<Result<_, String>>()?;
        Ok(FrontendOptions(entries))
    }
}

fn decode_entry(
    key: &str,
    value: Value,
    sources: &mut source_table::Imported<'_>,
    inventories: &mut ImportedInventories,
) -> Result<Entry, String> {
    if source_key(key)
        || inventories.shared_options.enabled && sharing::key(key)
        || !inventories.legacy() && key == crate::inherent_calls::UNIT_KEY
    {
        let encoded: KnownImport =
            serde_json::from_value(value).map_err(|error| error.to_string())?;
        encoded.decode(key, sources, inventories)
    } else if key == crate::inherent_calls::UNIT_KEY {
        Ok(inventories.legacy_entry(value))
    } else {
        Ok(Entry::new(key, value))
    }
}

impl KnownImport {
    fn decode(
        self,
        key: &str,
        sources: &mut source_table::Imported<'_>,
        inventories: &mut ImportedInventories,
    ) -> Result<Entry, String> {
        match self {
            Self::Inline(value) if inventories.shared_options.enabled && sharing::key(key) => {
                Ok(inventories.shared_options.retain(key, value))
            }
            Self::Inline(value) => Ok(Entry::new(key, value)),
            Self::SharedReference(index) if sharing::key(key) => {
                inventories.shared_options.entry(key, index)
            }
            Self::Census(value) if key == crate::nominal_boundary::census::KEY => {
                Ok(inventories.retain_census((*value).decode(sources)?))
            }
            Self::CensusReference(index) if key == crate::nominal_boundary::census::KEY => {
                inventories.census(index)
            }
            Self::Origins(value) if key == crate::declaration_origins::KEY => value.decode(sources),
            Self::InherentUnit(index) if key == crate::inherent_calls::UNIT_KEY => {
                inventories.entry(index)
            }
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
