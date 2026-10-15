//! Typed ownership for repeated data evidence, retaining the open JSON boundary.
pub(crate) mod bundle;
use crate::{concrete_types::InterfaceObservations, field_locations::FieldObservations};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    borrow::Cow,
    collections::BTreeMap,
    ops::Index,
    sync::{Arc, OnceLock},
};

/// Open frontend options with single owned typed interface/field records.
/// JSON compatibility access materializes only the requested record. Analysis
/// uses `data`; unknown or malformed evidence stays lossless.
#[derive(Clone, Debug, Default)]
pub struct FrontendOptions(BTreeMap<String, Entry>);

#[derive(Clone, Debug)]
enum Entry {
    Json(Value),
    Invalid { value: Value, error: String },
    Interface(Record<InterfaceObservations>),
    Fields(Record<FieldObservations>),
    Origins(Record<crate::declaration_origins::Origins>),
    Census(Record<Arc<crate::nominal_boundary::census::Census>>),
}

#[derive(Clone, Debug)]
struct Record<T> {
    value: T,
    // Optional null nominal identities must retain their original wire presence.
    nulls: Vec<String>,
    json: OnceLock<Value>,
}

impl<T: Serialize> Record<T> {
    fn new(value: T, nulls: Vec<String>) -> Self {
        Self {
            value,
            nulls,
            json: OnceLock::new(),
        }
    }

    fn render(&self) -> Value {
        let mut json = serde_json::to_value(&self.value).expect("typed evidence serializes");
        for path in &self.nulls {
            json.pointer_mut(path)
                .expect("nominal evidence parent")
                .as_object_mut()
                .expect("nominal evidence object")
                .insert("source_identity".into(), Value::Null);
        }
        json
    }

    fn json(&self) -> &Value {
        self.json.get_or_init(|| self.render())
    }

    fn into_json(mut self) -> Value {
        self.json.take().unwrap_or_else(|| self.render())
    }
}

impl Entry {
    fn new(key: &str, json: Value) -> Self {
        match key {
            crate::concrete_types::KEY => Self::decode(json, Self::Interface),
            crate::field_locations::KEY => Self::decode(json, Self::Fields),
            crate::nominal_boundary::census::KEY => Self::decode(json, Self::Census),
            crate::declaration_origins::KEY => Self::decode(json, Self::Origins),
            _ => Self::Json(json),
        }
    }

    fn decode<T: serde::de::DeserializeOwned + Serialize>(
        json: Value,
        wrap: impl FnOnce(Record<T>) -> Self,
    ) -> Self {
        let result = T::deserialize(&json);
        match result {
            Ok(value) => wrap(Record::new(value, nulls(&json))),
            Err(error) => Self::Invalid {
                value: json,
                error: error.to_string(),
            },
        }
    }

    fn json(&self) -> &Value {
        match self {
            Self::Json(value) | Self::Invalid { value, .. } => value,
            Self::Interface(record) => record.json(),
            Self::Fields(record) => record.json(),
            Self::Census(record) => record.json(),
            Self::Origins(record) => record.json(),
        }
    }

    fn equivalent(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Fields(left), Self::Fields(right)) => {
                left.value == right.value && left.nulls == right.nulls
            }
            (Self::Interface(left), Self::Interface(right)) => {
                left.value == right.value && left.nulls == right.nulls
            }
            (Self::Origins(left), Self::Origins(right)) => {
                left.value == right.value && left.nulls == right.nulls
            }
            (Self::Census(left), Self::Census(right)) => {
                left.value == right.value && left.nulls == right.nulls
            }
            _ => self.json() == other.json(),
        }
    }

    fn into_json(self) -> Value {
        match self {
            Self::Json(value) | Self::Invalid { value, .. } => value,
            Self::Interface(record) => record.into_json(),
            Self::Fields(record) => record.into_json(),
            Self::Census(record) => record.into_json(),
            Self::Origins(record) => record.into_json(),
        }
    }
}

impl Serialize for Entry {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Json(value) | Self::Invalid { value, .. } => value.serialize(serializer),
            Self::Interface(record) => record.render().serialize(serializer),
            Self::Fields(record) => record.render().serialize(serializer),
            Self::Census(record) => record.render().serialize(serializer),
            Self::Origins(record) => record.render().serialize(serializer),
        }
    }
}

impl FrontendOptions {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key).map(Entry::json)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        let entry = self.0.get_mut(key)?;
        if !matches!(entry, Entry::Json(_)) {
            let original = std::mem::replace(entry, Entry::Json(Value::Null));
            *entry = Entry::Json(original.into_json());
        }
        let Entry::Json(value) = entry else {
            unreachable!()
        };
        Some(value)
    }

    pub fn insert(&mut self, key: String, value: Value) -> Option<Value> {
        let entry = Entry::new(&key, value);
        self.0.insert(key, entry).map(Entry::into_json)
    }

    /// Move a registered record into this owning frontend context.
    pub fn insert_data<T: DataEvidence>(&mut self, value: T) {
        value.install(self);
    }

    /// Borrow a registered immutable record. The owned fallback supports JSON
    /// mutation adapters; frozen units normalize those entries before analysis.
    pub fn data<T: DataEvidence>(&self) -> Result<Cow<'_, T>, String> {
        match T::borrow(self) {
            Some(value) => Ok(Cow::Borrowed(value)),
            None => self.decode(T::KEY).map(Cow::Owned),
        }
    }

    fn decode<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<T, String> {
        if let Some(Entry::Invalid { error, .. }) = self.0.get(key) {
            return Err(error.clone());
        }
        let value = self
            .get(key)
            .ok_or_else(|| format!("Missing data evidence {key}"))?;
        T::deserialize(value).map_err(|error| error.to_string())
    }

    /// Re-establish typed ownership after compatibility JSON mutation, before
    /// freezing a unit. Invalid entries remain available with their exact error.
    pub(crate) fn normalize_data(&mut self) {
        for key in [
            crate::concrete_types::KEY,
            crate::field_locations::KEY,
            crate::nominal_boundary::census::KEY,
            crate::declaration_origins::KEY,
        ] {
            if matches!(self.0.get(key), Some(Entry::Json(_))) {
                let entry = self.0.remove(key).expect("existing data evidence");
                self.0
                    .insert(key.into(), Entry::new(key, entry.into_json()));
            }
        }
    }

    pub(crate) fn intern_census(
        &mut self,
        owners: &mut Vec<Arc<crate::nominal_boundary::census::Census>>,
    ) -> Result<(), String> {
        let Some(Entry::Census(record)) = self.0.get_mut(crate::nominal_boundary::census::KEY)
        else {
            return Ok(());
        };
        record.value = intern_owner(&record.value, owners)?;
        Ok(())
    }

    /// Share exact original bytes without conferring source verification authority.
    pub(crate) fn share_origin_sources(&mut self) -> Result<(), String> {
        let Some(Entry::Census(census)) = self.0.get(crate::nominal_boundary::census::KEY) else {
            return Ok(());
        };
        let census = Arc::clone(&census.value);
        let Some(Entry::Origins(origins)) = self.0.get_mut(crate::declaration_origins::KEY) else {
            return Ok(());
        };
        origins.value.share_sources(&census)
    }

    pub(crate) fn function_source(
        &self,
        source: &crate::SourceFacts,
        span: crate::ByteSpan,
    ) -> Result<crate::SourceText, String> {
        let Some(Entry::Census(census)) = self.0.get(crate::nominal_boundary::census::KEY) else {
            return Ok(source.bytes.clone());
        };
        let path = self
            .get("source_file")
            .and_then(Value::as_str)
            .ok_or("Missing captured function source-file identity")?;
        let file = census
            .value
            .files
            .iter()
            .find(|file| file.path == std::path::Path::new(path))
            .ok_or("Missing owned function source snapshot")?;
        source.bytes.in_snapshot(Arc::clone(&file.bytes), span)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.0.remove(key).map(Entry::into_json)
    }
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

fn intern_owner(
    census: &Arc<crate::nominal_boundary::census::Census>,
    owners: &mut Vec<Arc<crate::nominal_boundary::census::Census>>,
) -> Result<Arc<crate::nominal_boundary::census::Census>, String> {
    for owner in owners.iter() {
        if owner.context == census.context {
            if **owner != **census {
                return Err("Contradictory source snapshots in one selected-unit context".into());
            }
            return Ok(Arc::clone(owner));
        }
    }
    owners.push(Arc::clone(census));
    Ok(Arc::clone(census))
}

impl Extend<(String, Value)> for FrontendOptions {
    fn extend<T: IntoIterator<Item = (String, Value)>>(&mut self, iter: T) {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}

impl From<BTreeMap<String, Value>> for FrontendOptions {
    fn from(values: BTreeMap<String, Value>) -> Self {
        let mut options = Self::default();
        options.extend(values);
        options
    }
}

impl Index<&str> for FrontendOptions {
    type Output = Value;
    fn index(&self, key: &str) -> &Self::Output {
        self.get(key).expect("frontend option key")
    }
}

impl PartialEq for FrontendOptions {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self.0.iter().all(|(key, value)| {
                other
                    .0
                    .get(key)
                    .is_some_and(|entry| value.equivalent(entry))
            })
    }
}
impl Eq for FrontendOptions {}

impl Serialize for FrontendOptions {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(self.0.iter())
    }
}

impl<'de> Deserialize<'de> for FrontendOptions {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        BTreeMap::<String, Value>::deserialize(deserializer).map(Self::from)
    }
}

fn nulls(value: &Value) -> Vec<String> {
    let mut found = Vec::new();
    collect_nulls(value, "", &mut found);
    found
}

fn collect_nulls(value: &Value, path: &str, found: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if map.get("source_identity") == Some(&Value::Null) {
                found.push(path.into());
            }
            for (key, child) in map {
                let key = key.replace('~', "~0").replace('/', "~1");
                collect_nulls(child, &format!("{path}/{key}"), found);
            }
        }
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                collect_nulls(child, &format!("{path}/{index}"), found);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests;

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::InterfaceObservations {}
    impl Sealed for super::FieldObservations {}
    impl Sealed for crate::declaration_origins::Origins {}
    impl Sealed for std::sync::Arc<crate::nominal_boundary::census::Census> {}
}

/// The closed set of normalized data records. This selects the record's exact
/// key and kind; callers cannot substitute records from another option table.
pub trait DataEvidence: sealed::Sealed + serde::de::DeserializeOwned + Serialize + Clone {
    const KEY: &'static str;
    fn borrow(options: &FrontendOptions) -> Option<&Self>;
    fn install(self, options: &mut FrontendOptions);
}

impl DataEvidence for InterfaceObservations {
    const KEY: &'static str = crate::concrete_types::KEY;
    fn borrow(options: &FrontendOptions) -> Option<&Self> {
        match options.0.get(Self::KEY) {
            Some(Entry::Interface(record)) => Some(&record.value),
            _ => None,
        }
    }
    fn install(self, options: &mut FrontendOptions) {
        options.0.insert(
            Self::KEY.into(),
            Entry::Interface(Record::new(self, Vec::new())),
        );
    }
}

impl DataEvidence for FieldObservations {
    const KEY: &'static str = crate::field_locations::KEY;
    fn borrow(options: &FrontendOptions) -> Option<&Self> {
        match options.0.get(Self::KEY) {
            Some(Entry::Fields(record)) => Some(&record.value),
            _ => None,
        }
    }
    fn install(self, options: &mut FrontendOptions) {
        options.0.insert(
            Self::KEY.into(),
            Entry::Fields(Record::new(self, Vec::new())),
        );
    }
}

impl DataEvidence for Arc<crate::nominal_boundary::census::Census> {
    const KEY: &'static str = crate::nominal_boundary::census::KEY;
    fn borrow(options: &FrontendOptions) -> Option<&Self> {
        match options.0.get(Self::KEY) {
            Some(Entry::Census(record)) => Some(&record.value),
            _ => None,
        }
    }
    fn install(self, options: &mut FrontendOptions) {
        options.0.insert(
            Self::KEY.into(),
            Entry::Census(Record::new(self, Vec::new())),
        );
    }
}

impl DataEvidence for crate::declaration_origins::Origins {
    const KEY: &'static str = crate::declaration_origins::KEY;
    fn borrow(options: &FrontendOptions) -> Option<&Self> {
        match options.0.get(Self::KEY) {
            Some(Entry::Origins(record)) => Some(&record.value),
            _ => None,
        }
    }
    fn install(self, options: &mut FrontendOptions) {
        options.0.insert(
            Self::KEY.into(),
            Entry::Origins(Record::new(self, Vec::new())),
        );
    }
}
