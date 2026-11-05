//! A lossless, shared JSON-value arena, used identically by every codec.
//!
//! Sharing is exact structural equality, not a claim that two compiler entities
//! have the same identity. Original IDs, compilation context, ordering, absent
//! fields, unknown observations, and source bytes all survive reconstruction.

use prost::Message;
use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeTuple};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};

pub const VERSION: u32 = 1;
pub const NULL: u32 = 0;
pub const BOOL: u32 = 1;
pub const UNSIGNED: u32 = 2;
pub const SIGNED: u32 = 3;
pub const FLOAT: u32 = 4;
pub const STRING: u32 = 5;
pub const ARRAY: u32 = 6;
pub const OBJECT: u32 = 7;

#[derive(Clone, PartialEq, Eq, Message, Serialize, Deserialize)]
pub struct Archive {
    #[prost(uint32, tag = "1")]
    pub version: u32,
    #[prost(string, repeated, tag = "2")]
    pub strings: Vec<String>,
    #[prost(message, repeated, tag = "3")]
    pub values: Vec<Value>,
    #[prost(uint32, repeated, packed = "true", tag = "4")]
    pub edges: Vec<u32>,
    #[prost(message, repeated, tag = "5")]
    pub documents: Vec<Document>,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct Value {
    #[prost(uint32, tag = "1")]
    pub kind: u32,
    #[prost(uint64, tag = "2")]
    pub payload: u64,
    #[prost(uint32, tag = "3")]
    pub start: u32,
    #[prost(uint32, tag = "4")]
    pub length: u32,
}

#[derive(Clone, PartialEq, Eq, Message, Serialize, Deserialize)]
pub struct Document {
    #[prost(uint32, tag = "1")]
    pub path: u32,
    #[prost(uint32, tag = "2")]
    pub root: u32,
}

// Compact tuples avoid repeating field names in JSON. FlexBuffers receives the
// same tuples; neither codec is given an embedded serialization of another one.
impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut tuple = serializer.serialize_tuple(4)?;
        tuple.serialize_element(&self.kind)?;
        tuple.serialize_element(&self.payload)?;
        tuple.serialize_element(&self.start)?;
        tuple.serialize_element(&self.length)?;
        tuple.end()
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (kind, payload, start, length) = <(u32, u64, u32, u32)>::deserialize(deserializer)?;
        Ok(Self {
            kind,
            payload,
            start,
            length,
        })
    }
}

impl Archive {
    pub fn empty() -> Self {
        Self {
            version: VERSION,
            ..Self::default()
        }
    }

    pub fn children(&self, value: &Value) -> &[u32] {
        let start = value.start as usize;
        &self.edges[start..start + value.length as usize]
    }

    pub fn expand(&self, id: u32) -> Json {
        let value = &self.values[id as usize];
        match value.kind {
            NULL => Json::Null,
            BOOL => Json::Bool(value.payload != 0),
            UNSIGNED => Json::from(value.payload),
            SIGNED => Json::from(value.payload as i64),
            FLOAT => Json::from(f64::from_bits(value.payload)),
            STRING => Json::String(self.strings[value.payload as usize].clone()),
            ARRAY => Json::Array(
                self.children(value)
                    .iter()
                    .map(|&id| self.expand(id))
                    .collect(),
            ),
            OBJECT => Json::Object(
                self.children(value)
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| (self.strings[pair[0] as usize].clone(), self.expand(pair[1])))
                    .collect(),
            ),
            _ => unreachable!("validated archive value"),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != VERSION {
            return Err("Unsupported benchmark archive version".into());
        }
        for (index, value) in self.values.iter().enumerate() {
            if value.kind > OBJECT {
                return Err(format!("Unknown value kind at {index}"));
            }
            if value.kind == STRING && value.payload as usize >= self.strings.len() {
                return Err(format!("Invalid string reference at {index}"));
            }
            let end = (value.start as usize)
                .checked_add(value.length as usize)
                .ok_or("Range overflow")?;
            if end > self.edges.len() {
                return Err(format!("Invalid edge range at {index}"));
            }
            let children = self.children(value);
            if value.kind == ARRAY && children.iter().any(|&id| id as usize >= index) {
                return Err(format!("Non-topological array at {index}"));
            }
            if value.kind == OBJECT {
                if !children.len().is_multiple_of(2) {
                    return Err(format!("Odd object at {index}"));
                }
                for pair in children.as_chunks::<2>().0 {
                    if pair[0] as usize >= self.strings.len() || pair[1] as usize >= index {
                        return Err(format!("Invalid object reference at {index}"));
                    }
                }
            }
            if value.kind < ARRAY && (value.start != 0 || value.length != 0) {
                return Err(format!("Unexpected scalar edges at {index}"));
            }
        }
        for document in &self.documents {
            if document.path as usize >= self.strings.len()
                || document.root as usize >= self.values.len()
            {
                return Err("Invalid document reference".into());
            }
        }
        Ok(())
    }

    /// Codec-independent digest over every owned field. Outside timed decoding.
    pub fn digest(&self) -> String {
        let mut hash = Sha256::new();
        hash.update(b"cpg-storage-bench-arena-v1\0");
        hash.update(self.version.to_le_bytes());
        hash.update((self.strings.len() as u64).to_le_bytes());
        for string in &self.strings {
            hash.update((string.len() as u64).to_le_bytes());
            hash.update(string.as_bytes());
        }
        hash.update((self.values.len() as u64).to_le_bytes());
        for value in &self.values {
            hash.update(value.kind.to_le_bytes());
            hash.update(value.payload.to_le_bytes());
            hash.update(value.start.to_le_bytes());
            hash.update(value.length.to_le_bytes());
        }
        hash.update((self.edges.len() as u64).to_le_bytes());
        for edge in &self.edges {
            hash.update(edge.to_le_bytes());
        }
        hash.update((self.documents.len() as u64).to_le_bytes());
        for document in &self.documents {
            hash.update(document.path.to_le_bytes());
            hash.update(document.root.to_le_bytes());
        }
        format!("{:x}", hash.finalize())
    }
}
