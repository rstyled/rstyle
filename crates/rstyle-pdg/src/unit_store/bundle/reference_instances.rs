//! Strict reference constructor replay against an independently inspected owner.
#[cfg(test)]
mod tests;

use super::*;
use crate::reference_instances::Haystack;
use serde_json::Value;

const SCHEMA: &str = "rstyle.reference-constructor-sidecar/1";

#[derive(Serialize)]
struct Export<'a> {
    schema: &'static str,
    owner_namespace: &'a UnitNamespace,
    definition_ordinal: usize,
    haystack: &'static str,
    observation: &'a Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Import {
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
        let import: Import = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
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
        let instance = self
            .reference_instance
            .as_ref()
            .ok_or("Store is not a reference constructor instance")?;
        let (owner, ordinal, haystack) = self
            .reference_instance_origin()
            .ok_or("Missing original reference constructor owner")?;
        let observation = instance.observation();
        let original = owner
            .functions
            .get(ordinal)
            .ok_or("Missing original constructor")?;
        crate::verification_policy::strict(|| -> Result<(), String> {
            let checked = crate::reference_instances::verify_constructor_projection(
                original,
                observation,
                haystack,
            )?;
            if checked.facts() != instance.checked.facts()
                || checked.haystack() != instance.checked.haystack()
                || self.functions.as_slice() != std::slice::from_ref(checked.facts())
            {
                return Err("Reference sidecar differs from its retained checked instance".into());
            }
            self.validate_evidence()
        })?;

        let export = Export {
            schema: SCHEMA,
            owner_namespace: &owner.namespace,
            definition_ordinal: ordinal,
            haystack: haystack.spelling(),
            observation,
        };
        serde_json::to_writer(writer, &export).map_err(|error| error.to_string())
    }
}
