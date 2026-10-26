//! Raw compiler observations. None of these deserializable records is a proof.
use crate::{
    ByteSpan,
    data_library::Callee,
    generated_sources::capture::{Definition, Origin},
};
use serde::{Deserialize, Serialize};

pub const IDENTITY: &str = "rstyle.rust.id-reader-size/1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    pub file: String,
    pub span: ByteSpan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub size: u64,
    pub align: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub identity: String,
    pub definition: Definition,
    pub implementation: Definition,
    pub origin: Origin,
    pub implementation_origin: Origin,
    pub defining_expansion: String,
    pub owner: Definition,
    pub owner_source: Site,
    pub original: Definition,
    pub original_source: Site,
    pub original_implementation: Definition,
    pub original_implementation_source: Site,
    pub size_of: Callee,
    pub size_argument: Definition,
    pub size_argument_source: Site,
    pub layout: Layout,
    pub value: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Annotation {
    /// Function-relative complete original type coordinates, not normalized spelling.
    pub span: ByteSpan,
    pub length: ByteSpan,
    pub constant: Observation,
}

pub(super) fn shape(record: &Observation) -> Result<(), String> {
    if record.identity != IDENTITY
        || record.value != 4
        || record.layout != (Layout { size: 4, align: 4 })
        || record.defining_expansion.is_empty()
        || record.definition.path != format!("{}::SIZE", record.owner.path)
        || record.original.path != format!("{}::SIZE", record.size_argument.path)
    {
        return Err("ID reader size lacks its exact bounded four-byte initializer chain".into());
    }

    let definitions = [
        &record.definition,
        &record.implementation,
        &record.owner,
        &record.original,
        &record.original_implementation,
        &record.size_argument,
    ];
    for definition in definitions {
        if !definition.local
            || definition.path.is_empty()
            || definition.definition_identity.is_empty()
            || definition.crate_name.is_empty()
            || definition.stable_crate_id.is_empty()
            || definition.crate_name != record.owner.crate_name
            || definition.stable_crate_id != record.owner.stable_crate_id
        {
            return Err("ID reader size definitions differ from their exact local crate".into());
        }
    }
    let mut identities = std::collections::BTreeSet::new();
    for definition in definitions {
        if !identities.insert(&definition.definition_identity) {
            return Err("ID reader size chain reuses a distinct definition identity".into());
        }
    }
    Ok(())
}
