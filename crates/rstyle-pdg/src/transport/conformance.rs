use crate::Conformance;
use serde::{Deserializer, Serializer};

pub(super) fn serialize<S: Serializer>(
    values: &[Conformance],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let [value] = values else {
        return Err(serde::ser::Error::custom(
            "expected one conformance assessment",
        ));
    };
    super::wire::Conformance::serialize(value, serializer)
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Conformance>, D::Error> {
    super::wire::Conformance::deserialize(deserializer).map(|value| vec![value])
}
