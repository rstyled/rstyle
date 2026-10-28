use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

pub(super) trait Record: Sized {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;
    fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>;
}

macro_rules! record {
    ($($name:ident),+) => {$(
        impl Record for crate::$name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                super::wire::$name::serialize(self, serializer)
            }
            fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                super::wire::$name::deserialize(deserializer)
            }
        }
    )+};
}
record!(PdgNode, PdgEdge, MemoryLocation, Capability);

struct Borrowed<'a, T>(&'a T);
struct Owned<T>(T);

impl<T: Record> Serialize for Borrowed<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Record> Deserialize<'de> for Owned<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self)
    }
}

pub(super) fn serialize<T: Record, S: Serializer>(
    values: &[T],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(values.iter().map(Borrowed))
}

pub(super) fn deserialize<'de, T: Record, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<T>, D::Error> {
    Vec::<Owned<T>>::deserialize(deserializer)
        .map(|values| values.into_iter().map(|v| v.0).collect())
}

pub(super) mod capabilities {
    use super::*;
    use crate::Capability;

    pub(crate) fn serialize<S: Serializer>(
        values: &BTreeMap<String, Capability>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_map(values.iter().map(|(key, value)| (key, Borrowed(value))))
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<String, Capability>, D::Error> {
        BTreeMap::<String, Owned<Capability>>::deserialize(deserializer).map(|values| {
            values
                .into_iter()
                .map(|(key, value)| (key, value.0))
                .collect()
        })
    }
}
