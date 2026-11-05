use crate::model::{self, Archive, Document, Value};
use serde::Serialize;
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    path::Path,
};

#[derive(Default, Serialize)]
pub struct Stats {
    pub source_bytes: u64,
    pub source_documents: usize,
    pub source_functions: usize,
    pub source_nodes: usize,
    pub input_values: u64,
    pub input_strings: u64,
    pub input_string_bytes: u64,
    pub unique_values: usize,
    pub unique_strings: usize,
    pub unique_string_bytes: usize,
    pub table_occurrences: HashMap<String, u64>,
    pub table_unique_records: HashMap<String, usize>,
}

pub struct Normalizer {
    pub archive: Archive,
    pub stats: Stats,
    strings: HashMap<u64, Vec<u32>>,
    values: HashMap<u64, Vec<u32>>,
    tables: HashMap<String, std::collections::HashSet<[u8; 32]>>,
}

impl Normalizer {
    pub fn new() -> Self {
        Self {
            archive: Archive::empty(),
            stats: Stats::default(),
            strings: HashMap::new(),
            values: HashMap::new(),
            tables: HashMap::new(),
        }
    }

    pub fn add(&mut self, path: &Path) -> Result<(), String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let json: Json =
            serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        self.stats.source_bytes += bytes.len() as u64;
        self.stats.source_documents += 1;
        drop(bytes);
        self.count_cpg(&json)?;
        let root = self.value(&json)?;
        // Prove exact JSON semantics before discarding the input document.
        if self.archive.expand(root) != json {
            return Err(format!("Normalization changed {}", path.display()));
        }
        let path = self.string(&path.to_string_lossy())?;
        self.archive.documents.push(Document { path, root });
        Ok(())
    }

    pub fn finish(mut self) -> Result<(Archive, Stats), String> {
        self.archive.validate()?;
        self.stats.unique_values = self.archive.values.len();
        self.stats.unique_strings = self.archive.strings.len();
        self.stats.unique_string_bytes = self.archive.strings.iter().map(String::len).sum();
        self.stats.table_unique_records =
            self.tables.into_iter().map(|(k, v)| (k, v.len())).collect();
        Ok((self.archive, self.stats))
    }

    fn count_cpg(&mut self, json: &Json) -> Result<(), String> {
        let Some(unit) = json.get("unit") else {
            return Ok(());
        };
        if let Some(functions) = unit.get("functions").and_then(Json::as_array) {
            self.stats.source_functions += functions.len();
            self.stats.source_nodes += functions
                .iter()
                .filter_map(|f| f.get("nodes")?.as_array())
                .map(Vec::len)
                .sum::<usize>();
        }
        for name in [
            "files",
            "definitions",
            "types",
            "values",
            "imports",
            "namespaces",
        ] {
            let Some(records) = unit.get(name).and_then(Json::as_array) else {
                continue;
            };
            *self.stats.table_occurrences.entry(name.into()).or_default() += records.len() as u64;
            let unique = self.tables.entry(name.into()).or_default();
            for record in records {
                let digest: [u8; 32] =
                    Sha256::digest(serde_json::to_vec(record).map_err(|e| e.to_string())?).into();
                unique.insert(digest);
            }
        }
        Ok(())
    }

    fn string(&mut self, value: &str) -> Result<u32, String> {
        self.stats.input_strings += 1;
        self.stats.input_string_bytes += value.len() as u64;
        let mut hash = DefaultHasher::new();
        value.hash(&mut hash);
        let hash = hash.finish();
        if let Some(ids) = self.strings.get(&hash)
            && let Some(&id) = ids
                .iter()
                .find(|&&id| self.archive.strings[id as usize] == value)
        {
            return Ok(id);
        }
        let id = u32::try_from(self.archive.strings.len()).map_err(|_| "Too many strings")?;
        self.archive.strings.push(value.to_owned());
        self.strings.entry(hash).or_default().push(id);
        Ok(id)
    }

    fn value(&mut self, json: &Json) -> Result<u32, String> {
        self.stats.input_values += 1;
        let mut children = Vec::new();
        let (kind, payload) = match json {
            Json::Null => (model::NULL, 0),
            Json::Bool(value) => (model::BOOL, u64::from(*value)),
            Json::Number(n) => match (n.as_u64(), n.as_i64()) {
                (Some(n), _) => (model::UNSIGNED, n),
                (_, Some(n)) => (model::SIGNED, n as u64),
                _ => (
                    model::FLOAT,
                    n.as_f64().ok_or("Invalid JSON number")?.to_bits(),
                ),
            },
            Json::String(s) => (model::STRING, self.string(s)? as u64),
            Json::Array(values) => {
                children.reserve(values.len());
                for value in values {
                    children.push(self.value(value)?);
                }
                (model::ARRAY, 0)
            }
            Json::Object(values) => {
                children.reserve(values.len() * 2);
                for (key, value) in values {
                    children.push(self.string(key)?);
                    children.push(self.value(value)?);
                }
                (model::OBJECT, 0)
            }
        };
        let mut hasher = DefaultHasher::new();
        (kind, payload, &children).hash(&mut hasher);
        let hash = hasher.finish();
        if let Some(ids) = self.values.get(&hash)
            && let Some(&id) = ids.iter().find(|&&id| {
                let value = &self.archive.values[id as usize];
                value.kind == kind
                    && value.payload == payload
                    && self.archive.children(value) == children
            })
        {
            return Ok(id);
        }
        let id = u32::try_from(self.archive.values.len()).map_err(|_| "Too many values")?;
        let length = u32::try_from(children.len()).map_err(|_| "Too many children")?;
        let start = if length == 0 {
            0
        } else {
            u32::try_from(self.archive.edges.len()).map_err(|_| "Too many edges")?
        };
        self.archive.edges.extend(children);
        self.archive.values.push(Value {
            kind,
            payload,
            start,
            length,
        });
        self.values.entry(hash).or_default().push(id);
        Ok(id)
    }
}
