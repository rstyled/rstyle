//! Strict original From replay includes its exact constructor proof inputs.
#[cfg(test)]
mod tests;

use super::*;
use crate::reference_instances::{Haystack, ReferenceObservation};
use serde_json::Value;

const SCHEMA: &str = "rstyle.reference-forwarder-sidecar/1";

#[derive(Serialize)]
struct Export<'a> {
    schema: &'static str,
    owner_namespace: &'a UnitNamespace,
    definition_ordinal: usize,
    haystack: &'static str,
    observation: &'a Value,
    constructor: super::reference_instances::Export<'a>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Import {
    schema: String,
    owner_namespace: UnitNamespace,
    definition_ordinal: usize,
    haystack: String,
    observation: Value,
    constructor: super::reference_instances::Import,
}

/// One independently inspected original owner serves both exact proof replays.
pub struct ReferenceForwarderReplay {
    constructor: ReferenceConstructorReplay,
}

impl ReferenceForwarderReplay {
    pub fn new(
        owner: Arc<UnitStore>,
        inspect: impl FnOnce(&[HirFnFacts]) -> Result<(), String>,
    ) -> Result<Self, String> {
        Ok(Self::from_constructor(ReferenceConstructorReplay::new(
            owner, inspect,
        )?))
    }

    /// Reuse an already inspected immutable owner; every replay still verifies
    /// both instance proofs and graphs strictly.
    pub fn from_constructor(constructor: ReferenceConstructorReplay) -> Self {
        Self { constructor }
    }

    pub fn owner(&self) -> &Arc<UnitStore> {
        self.constructor.owner()
    }

    /// Recompute both bodies, the exact call link and graph under strict policy.
    pub fn replay(&self, bytes: &[u8]) -> Result<UnitStore, String> {
        let import: Import = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if import.schema != SCHEMA || import.owner_namespace != self.owner().namespace {
            return Err("From sidecar differs from its independently selected owner".into());
        }
        let haystack = argument(&import.haystack)?;
        crate::verification_policy::strict(|| {
            let constructor = Arc::new(self.constructor.replay_inputs(import.constructor)?);
            if constructor.reference_instance_origin().map(|(_, _, ty)| ty) != Some(haystack) {
                return Err("From sidecar changes its exact constructor substitution".into());
            }
            let store = UnitStore::capture_reference_forwarder(
                Arc::clone(self.owner()),
                import.definition_ordinal,
                &import.observation,
                constructor,
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
        _ => Err("From sidecar requires concrete str/[u8] substitution".into()),
    }
}

impl UnitStore {
    /// Export only original proof inputs after replaying both complete proofs.
    pub fn write_reference_forwarder_sidecar(&self, writer: impl Write) -> Result<(), String> {
        let export = crate::verification_policy::strict(|| self.forwarder_export())?;
        serde_json::to_writer(writer, &export).map_err(|error| error.to_string())
    }

    fn forwarder_export(&self) -> Result<Export<'_>, String> {
        let record = self
            .reference_forwarder
            .as_ref()
            .ok_or("Store is not a reference forwarder")?;
        let [constructor] = self.dependencies.as_slice() else {
            return Err("From sidecar requires its sole checked constructor dependency".into());
        };

        self.verify_forwarder_dependency(constructor)?;
        let constructor = constructor.reference_constructor_inputs()?;
        self.validate_evidence()?;
        Ok(Export {
            schema: SCHEMA,
            owner_namespace: &record.owner.namespace,
            definition_ordinal: record.ordinal,
            haystack: record.checked.constructor().haystack().spelling(),
            observation: &record.observation,
            constructor,
        })
    }

    fn verify_forwarder_dependency(&self, constructor: &UnitStore) -> Result<(), String> {
        let record = self
            .reference_forwarder
            .as_ref()
            .ok_or("Missing From owner")?;
        let (owner, ordinal, haystack) = constructor
            .reference_instance_origin()
            .ok_or("From sidecar dependency lacks its original constructor owner")?;
        if !Arc::ptr_eq(owner, &record.owner) || haystack != record.checked.constructor().haystack()
        {
            return Err("From sidecar constructor changes its original owner/substitution".into());
        }

        let original = record
            .owner
            .functions
            .get(record.ordinal)
            .ok_or("Missing original From")?;
        let target = owner
            .functions
            .get(ordinal)
            .ok_or("Missing original constructor")?;
        let observation = constructor
            .reference_instance
            .as_ref()
            .ok_or("Missing checked constructor")?
            .observation();
        let checked = crate::reference_instances::verify_forwarder_projection(
            ReferenceObservation {
                canonical: original,
                observation: &record.observation,
            },
            ReferenceObservation {
                canonical: target,
                observation,
            },
            haystack,
        )?;
        self.verify_forwarder_sites(&checked, constructor)
    }

    fn verify_forwarder_sites(
        &self,
        checked: &crate::reference_instances::CheckedForwarderProjection,
        constructor: &UnitStore,
    ) -> Result<(), String> {
        let retained = &self
            .reference_forwarder
            .as_ref()
            .ok_or("Missing From owner")?
            .checked;
        if checked.facts() != retained.facts()
            || checked.call() != retained.call()
            || checked.constructor().facts() != retained.constructor().facts()
            || self.functions.as_slice() != std::slice::from_ref(checked.facts())
            || constructor.functions.as_slice()
                != std::slice::from_ref(checked.constructor().facts())
        {
            return Err("From sidecar differs from its retained exact proof/facts".into());
        }

        let [links] = self.external_links.as_slice() else {
            return Err("From sidecar loses its exact forwarding call inventory".into());
        };
        if links.len() != 1
            || !links
                .get(&site(checked.call()))
                .is_some_and(|target| target.dependency == 0 && target.function == FunctionId(0))
        {
            return Err("From sidecar changes its exact constructor call link".into());
        }
        Ok(())
    }
}
