//! Node identities remain named values so existing alpha renaming can see them.
use super::*;
use serde::{Deserializer, Serializer};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record<T> {
    pdg_node_id: String,
    value: T,
}

pub(super) fn serialize<S>(
    values: &BTreeMap<String, Value>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let records: Vec<_> = values
        .iter()
        .map(|(id, value)| Record {
            pdg_node_id: id.clone(),
            value,
        })
        .collect();
    records.serialize(serializer)
}

pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<String, Value>, D::Error>
where
    D: Deserializer<'de>,
{
    let records = Vec::<Record<Value>>::deserialize(deserializer)?;
    let mut values = BTreeMap::new();
    for record in records {
        if values.insert(record.pdg_node_id, record.value).is_some() {
            return Err(serde::de::Error::custom(
                "Duplicate digit-product origin node identity",
            ));
        }
    }

    Ok(values)
}
