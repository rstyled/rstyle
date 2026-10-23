//! Strict replay of projections against one independently inspected original owner.
use super::*;
use crate::scalar_instances::Projection;

const SCHEMA: &str = "rstyle.scalar-instance-sidecar/1";

#[derive(Serialize)]
struct Export<'a> {
    schema: &'static str,
    owner_namespace: &'a UnitNamespace,
    definition_ordinal: usize,
    projection: &'a Projection,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Import {
    schema: String,
    owner_namespace: UnitNamespace,
    definition_ordinal: usize,
    projection: Projection,
}

/// One original immutable owner, independently inspected and strictly checked once.
/// Replaying a body grants no association to any caller or emitted artifact.
pub struct ScalarInstanceReplay {
    owner: Arc<UnitStore>,
}

pub(super) fn checked_original_owner(
    owner: Arc<UnitStore>,
    inspect: impl FnOnce(&[HirFnFacts]) -> Result<(), String>,
) -> Result<Arc<UnitStore>, String> {
    if owner.has_instance() || !owner.dependencies.is_empty() {
        return Err("Instance replay requires an original unlinked definition owner".into());
    }
    inspect(&owner.functions)?;
    crate::verification_policy::strict(|| owner.validate_evidence())?;

    Ok(owner)
}

impl ScalarInstanceReplay {
    pub fn new(
        owner: Arc<UnitStore>,
        inspect: impl FnOnce(&[HirFnFacts]) -> Result<(), String>,
    ) -> Result<Self, String> {
        Ok(Self {
            owner: checked_original_owner(owner, inspect)?,
        })
    }

    /// Decode structural proof inputs and recompute the body and graph strictly.
    /// No serialized graph, certificate or verification-policy token is accepted.
    pub fn replay(&self, bytes: &[u8]) -> Result<UnitStore, String> {
        let import: Import = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if import.schema != SCHEMA || import.owner_namespace != self.owner.namespace {
            return Err("Scalar sidecar differs from its independently selected owner".into());
        }

        crate::verification_policy::strict(|| {
            UnitStore::capture_scalar_instance(
                Arc::clone(&self.owner),
                import.definition_ordinal,
                &import.projection,
            )
        })
    }

    pub fn owner(&self) -> &Arc<UnitStore> {
        &self.owner
    }
}

impl UnitStore {
    /// Export proof inputs only after independently rechecking the retained origin.
    /// Original definition ordinals and the ordinary bundle format remain unchanged.
    pub fn write_scalar_instance_sidecar(
        &self,
        writer: impl Write,
        projection: &Projection,
    ) -> Result<(), String> {
        write_inputs(writer, self.scalar_instance_inputs(projection))
    }

    fn scalar_instance_inputs<'a>(
        &'a self,
        projection: &'a Projection,
    ) -> Result<Export<'a>, String> {
        let input = super::super::instances::retained_input::<
            super::super::instances::InstanceRecord,
        >(self)?;
        let instance = input.record;
        let original = input.original;
        crate::verification_policy::strict(|| -> Result<(), String> {
            let checked = crate::scalar_instances::verify(original, projection)?;
            crate::scalar_instances::verify_owner(original, &checked)?;
            if checked.facts() != instance.checked.facts()
                || checked.scalar_argument() != instance.checked.scalar_argument()
                || checked.comparison_count() != instance.checked.comparison_count()
            {
                return Err(
                    "Scalar sidecar projection differs from the retained checked instance".into(),
                );
            }
            Ok(())
        })?;

        Ok(Export {
            schema: SCHEMA,
            owner_namespace: &input.owner.namespace,
            definition_ordinal: input.ordinal,
            projection,
        })
    }
}

#[cfg(test)]
mod tests;

pub(super) fn decode_and_replay<I: serde::de::DeserializeOwned>(
    bytes: &[u8],
    replay: impl FnOnce(I) -> Result<UnitStore, String>,
) -> Result<UnitStore, String> {
    let import = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    replay(import)
}

pub(super) fn write_inputs<T: Serialize>(
    writer: impl Write,
    inputs: Result<T, String>,
) -> Result<(), String> {
    let export = inputs?;
    serde_json::to_writer(writer, &export).map_err(|error| error.to_string())
}

pub(super) fn checked_constructor_input<R: super::super::instances::RetainedConstructor>(
    store: &UnitStore,
) -> Result<super::super::instances::RetainedInput<'_, R>, String> {
    let input = super::super::instances::retained_input::<R>(store)?;
    crate::verification_policy::strict(|| {
        input.record.verify_store(store)?;
        input.record.reverify(input.original, &store.functions)?;
        store.validate_evidence()
    })?;
    Ok(input)
}

pub(super) trait ConstructorSidecar: super::super::instances::RetainedConstructor {
    type Export<'a>: Serialize
    where
        Self: 'a;
    fn export<'a>(input: super::super::instances::RetainedInput<'a, Self>) -> Self::Export<'a>;
}

pub(super) fn constructor_inputs<R: ConstructorSidecar>(
    store: &UnitStore,
) -> Result<R::Export<'_>, String> {
    checked_constructor_input::<R>(store).map(R::export)
}

pub(super) fn write_constructor<R: ConstructorSidecar>(
    store: &UnitStore,
    writer: impl Write,
) -> Result<(), String> {
    write_inputs(writer, constructor_inputs::<R>(store))
}
