//! Complete typed keys interned inside one pair; wire IDs carry no content identity.
use super::*;
use crate::{MemoryLocation, data_types::TypeToken, producer::PairView, unit_store::EvidenceFacts};
use serde::{Deserialize, Serialize};

const ABSTRACTION: &str = "rstyle.formal-field/2";

fn abstraction(path: &[Projection]) -> &'static str {
    path.iter()
        .rev()
        .find_map(|step| match step {
            Projection::ByteWindow { .. } => Some("rstyle.formal-byte-window/1"),
            Projection::IndexedElement { .. } => Some("rstyle.formal-indexed-element/1"),
            _ => None,
        })
        .unwrap_or(ABSTRACTION)
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LocationKey {
    parameter: u32,
    root_symbol: String,
    path: Vec<Projection>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Record {
    parameter: u32,
    root_symbol: String,
    path: Vec<Projection>,
    #[serde(rename = "type")]
    value: TypeToken,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct LocationId(u32);

impl LocationId {
    fn wire(self) -> String {
        format!("location:local:{}", self.0)
    }

    fn from_wire(value: &str, count: usize) -> Result<Self, String> {
        let id = value
            .strip_prefix("location:local:")
            .and_then(|value| value.parse::<u32>().ok())
            .map(Self)
            .ok_or("Invalid local location ID")?;
        if id.0 as usize >= count || id.wire() != value {
            return Err("Local location ID is noncanonical or outside its pair".into());
        }
        Ok(id)
    }
}

struct Entry {
    id: LocationId,
    value: TypeToken,
}

#[derive(Default)]
struct LocationTable {
    keys: BTreeMap<LocationKey, Entry>,
    imported: bool,
}

impl LocationTable {
    fn from_locations(locations: &[MemoryLocation]) -> Result<Self, String> {
        let mut table = Self::default();
        let mut ids = BTreeSet::new();
        for location in locations {
            let id = LocationId::from_wire(&location.id, locations.len())?;
            if !ids.insert(id) {
                return Err("Unsupported or duplicate local location record".into());
            }
            let record = Record::deserialize(
                location
                    .extensions
                    .get(LOCATION_KEY)
                    .ok_or("Missing typed location record")?,
            )
            .map_err(|error| error.to_string())?;
            if location.abstraction != abstraction(&record.path) {
                return Err(
                    "Location abstraction disagrees with its projection interpretation".into(),
                );
            }
            let key = LocationKey {
                parameter: record.parameter,
                root_symbol: record.root_symbol,
                path: record.path,
            };
            if table
                .keys
                .insert(
                    key,
                    Entry {
                        id,
                        value: record.value,
                    },
                )
                .is_some()
            {
                return Err("Duplicate complete location key".into());
            }
        }

        table.imported = true;
        Ok(table)
    }

    fn intern(&mut self, key: &LocationKey, value: &TypeToken) -> Result<LocationId, String> {
        if let Some(entry) = self.keys.get(key) {
            return if entry.value == *value {
                Ok(entry.id)
            } else {
                Err("One formal location has contradictory type evidence".into())
            };
        }
        if self.imported {
            return Err("Source location has no imported local handle".into());
        }

        let id =
            LocationId(u32::try_from(self.keys.len()).map_err(|_| "Oversized location table")?);
        self.keys.insert(
            key.clone(),
            Entry {
                id,
                value: value.clone(),
            },
        );
        Ok(id)
    }
}

/// The only handle lookup lives in this owner. No caller can resolve a foreign ID.
pub(crate) struct LocationContext<'a> {
    pub facts: &'a dyn EvidenceFacts,
    table: LocationTable,
}

impl<'a> LocationContext<'a> {
    pub fn new(facts: &'a dyn EvidenceFacts) -> Self {
        Self {
            facts,
            table: LocationTable::default(),
        }
    }

    pub fn from_pair(pair: PairView<'_>, facts: &'a dyn EvidenceFacts) -> Result<Self, String> {
        Ok(Self {
            facts,
            table: LocationTable::from_locations(&pair.pdg.memory_locations)?,
        })
    }

    pub fn read_accesses(
        &mut self,
        locations: Vec<crate::field_locations::derived::Located>,
    ) -> Result<(Vec<crate::Access>, BTreeMap<String, MemoryLocation>), String> {
        let entries = locations
            .into_iter()
            .map(|location| {
                self.formal_location(
                    location.parameter,
                    &location.path,
                    TypeToken::from_tree(&location.value),
                )
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let reads = entries
            .keys()
            .map(|id| crate::Access {
                symbol_id: None,
                location_id: Some(id.clone()),
                unresolved_reason: None,
                operand_position: Some(0),
            })
            .collect();

        Ok((reads, entries))
    }

    pub fn formal_location(
        &mut self,
        parameter: u32,
        path: &[Projection],
        value: TypeToken,
    ) -> Result<(String, MemoryLocation), String> {
        let root = self
            .facts
            .signature
            .params
            .get(parameter as usize)
            .ok_or("Unknown formal location root")?
            .symbol
            .clone();
        let key = LocationKey {
            parameter,
            root_symbol: root,
            path: path.to_vec(),
        };
        let id = self.table.intern(&key, &value)?.wire();

        let location = MemoryLocation {
            id: id.clone(),
            abstraction: abstraction(&key.path).into(),
            description: "Read-only formal projection; shared roots may alias".into(),
            evidence: crate::Capability {
                policy: Some(
                    crate::extraction_profile::ExtractionProfile::from_facts(self.facts)?.policy(),
                ),
                status: crate::CapabilityStatus::Supported,
                description: "Independent source projection and immutable concrete type proof"
                    .into(),
            },
            extensions: BTreeMap::from([(
                LOCATION_KEY.into(),
                serde_json::to_value(Record {
                    parameter: key.parameter,
                    root_symbol: key.root_symbol,
                    path: key.path,
                    value,
                })
                .map_err(|error| error.to_string())?,
            )]),
        };
        Ok((id, location))
    }
}

/// Translate local handles by complete typed record equality for strict replay.
pub(crate) fn correspondence(
    actual: PairView<'_>,
    expected: PairView<'_>,
) -> Result<BTreeMap<String, String>, String> {
    let actual = LocationTable::from_locations(&actual.pdg.memory_locations)?;
    let expected = LocationTable::from_locations(&expected.pdg.memory_locations)?;
    if actual.keys.len() != expected.keys.len() {
        return Err("Location inventory differs from source".into());
    }
    actual
        .keys
        .iter()
        .map(|(key, entry)| {
            let other = expected
                .keys
                .get(key)
                .filter(|other| other.value == entry.value)
                .ok_or("Location key or type differs from source")?;
            Ok((entry.id.wire(), other.id.wire()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(parameter: u32, path: Vec<Projection>) -> LocationKey {
        LocationKey {
            parameter,
            root_symbol: format!("formal:{parameter}"),
            path,
        }
    }

    fn record(id: u32, key: LocationKey) -> MemoryLocation {
        MemoryLocation {
            id: LocationId(id).wire(),
            abstraction: ABSTRACTION.into(),
            description: String::new(),
            evidence: crate::Capability {
                policy: None,
                status: crate::CapabilityStatus::Supported,
                description: String::new(),
            },
            extensions: BTreeMap::from([(
                LOCATION_KEY.into(),
                serde_json::to_value(Record {
                    parameter: key.parameter,
                    root_symbol: key.root_symbol,
                    path: key.path,
                    value: TypeToken::Scalar("u8".into()),
                })
                .unwrap(),
            )]),
        }
    }

    #[test]
    fn complete_keys_preserve_roots_projection_order_and_contradictions() {
        let mut table = LocationTable::default();
        let value = TypeToken::Scalar("u8".into());
        let path = vec![
            Projection::SharedReferent,
            Projection::TupleField { position: 0 },
        ];
        let first = key(0, path.clone());
        let id = table.intern(&first, &value).unwrap();
        assert_eq!(table.intern(&first, &value).unwrap(), id);
        assert_ne!(table.intern(&key(1, path.clone()), &value).unwrap(), id);
        let reversed = key(0, path.into_iter().rev().collect());
        assert_ne!(table.intern(&reversed, &value).unwrap(), id);
        let mut different_symbol = first.clone();
        different_symbol.root_symbol = "other formal".into();
        assert_ne!(table.intern(&different_symbol, &value).unwrap(), id);
        assert!(
            table
                .intern(&first, &TypeToken::Scalar("bool".into()))
                .is_err()
        );
    }

    #[test]
    fn import_uses_exact_keys_and_rejects_foreign_duplicate_and_sparse_handles() {
        let first = key(0, vec![Projection::SharedReferent]);
        let second = key(1, vec![Projection::SharedReferent]);
        let locations = [record(1, first.clone()), record(0, second.clone())];
        let mut imported = LocationTable::from_locations(&locations).unwrap();
        let value = TypeToken::Scalar("u8".into());
        assert_eq!(imported.intern(&first, &value).unwrap(), LocationId(1));
        assert_eq!(imported.intern(&second, &value).unwrap(), LocationId(0));
        let mut other = LocationTable::from_locations(&[record(0, second.clone())]).unwrap();
        assert!(other.intern(&first, &value).is_err());
        assert!(
            LocationTable::from_locations(&[record(0, first.clone()), record(0, second.clone())])
                .is_err()
        );
        assert!(
            LocationTable::from_locations(&[record(0, first.clone()), record(1, first.clone())])
                .is_err()
        );
        assert!(
            LocationTable::from_locations(&[record(2, first.clone()), record(0, second)]).is_err()
        );
        let mut obsolete = record(0, first);
        obsolete.abstraction = "rstyle.formal-field/1".into();
        assert!(LocationTable::from_locations(&[obsolete]).is_err());
    }
}
