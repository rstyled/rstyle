//! Strict reference constructor replay against an independently inspected owner.
#[cfg(test)]
mod tests;

use super::super::instances::RetainedConstructor;
use super::super::reference_instances::ReferenceInstanceRecord;
use super::*;
use crate::reference_instances::Haystack;
use serde_json::Value;

const SCHEMA: &str = "rstyle.reference-constructor-sidecar/1";

#[derive(Serialize)]
pub(super) struct Export<'a> {
    schema: &'static str,
    owner_namespace: &'a UnitNamespace,
    definition_ordinal: usize,
    haystack: &'static str,
    observation: &'a Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Import {
    schema: String,
    owner_namespace: UnitNamespace,
    definition_ordinal: usize,
    haystack: String,
    observation: Value,
}

/// Replay accepts proof inputs, never serialized graphs or activation tokens.
/// Neither this session nor its result associates the constructor with a caller.
pub struct ReferenceConstructorReplay {
    owner: Arc<UnitStore>,
}

impl ReferenceConstructorReplay {
    pub fn new(
        owner: Arc<UnitStore>,
        inspect: impl FnOnce(&[HirFnFacts]) -> Result<(), String>,
    ) -> Result<Self, String> {
        Ok(Self {
            owner: super::instances::checked_original_owner(owner, inspect)?,
        })
    }

    pub fn owner(&self) -> &Arc<UnitStore> {
        &self.owner
    }

    /// Recompute the source, effects, origins and graph under strict policy.
    pub fn replay(&self, bytes: &[u8]) -> Result<UnitStore, String> {
        super::instances::decode_and_replay(bytes, |import| self.replay_inputs(import))
    }

    pub(super) fn replay_inputs(&self, import: Import) -> Result<UnitStore, String> {
        if import.schema != SCHEMA || import.owner_namespace != self.owner.namespace {
            return Err("Reference sidecar differs from its independently selected owner".into());
        }
        let haystack = argument(&import.haystack)?;

        crate::verification_policy::strict(|| {
            let store = UnitStore::capture_reference_constructor(
                Arc::clone(&self.owner),
                import.definition_ordinal,
                &import.observation,
                haystack,
            )?;
            store.validate_evidence()?;
            Ok(store)
        })
    }
}

fn argument(value: &str) -> Result<Haystack, String> {
    match value {
        "str" => Ok(Haystack::Str),
        "[u8]" => Ok(Haystack::Bytes),
        _ => Err("Reference sidecar requires concrete str/[u8] substitution".into()),
    }
}

impl UnitStore {
    /// Export the exact retained proof inputs after independently rechecking them.
    pub fn write_reference_constructor_sidecar(&self, writer: impl Write) -> Result<(), String> {
        super::instances::write_constructor::<ReferenceInstanceRecord>(self, writer)
    }

    pub(super) fn reference_constructor_inputs(&self) -> Result<Export<'_>, String> {
        super::instances::constructor_inputs::<ReferenceInstanceRecord>(self)
    }
}

impl super::instances::ConstructorSidecar for ReferenceInstanceRecord {
    type Export<'a> = Export<'a>;

    fn export<'a>(input: super::super::instances::RetainedInput<'a, Self>) -> Export<'a> {
        Export {
            schema: SCHEMA,
            owner_namespace: &input.owner.namespace,
            definition_ordinal: input.ordinal,
            haystack: input.argument.spelling(),
            observation: input.record.observation(),
        }
    }
}
