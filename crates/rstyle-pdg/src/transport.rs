//! Current shared PDG wire boundary. Internal records retain historical policy
//! metadata for strict verification; encoding checks it before projecting the
//! single document profile. Namespaced annotations are never rewritten here.
mod conformance;
mod policies;
mod records;
mod wire;

use crate::PdgDocument;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize)]
pub struct Borrowed<'a>(#[serde(serialize_with = "serialize")] pub &'a PdgDocument);

#[derive(Serialize, Deserialize)]
pub struct Document(#[serde(with = "self")] pub PdgDocument);

pub fn serialize<S: Serializer>(doc: &PdgDocument, serializer: S) -> Result<S::Ok, S::Error> {
    policies::check(doc).map_err(serde::ser::Error::custom)?;
    wire::Document::serialize(doc, serializer)
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PdgDocument, D::Error> {
    let doc = wire::Document::deserialize(deserializer)?;
    let count = &doc.function.reference_count;
    crate::model::FactShape::new(count.status, count.value.is_some(), count.reason.as_deref())
        .check("count")
        .map_err(serde::de::Error::custom)?;

    Ok(policies::inherit(doc))
}
