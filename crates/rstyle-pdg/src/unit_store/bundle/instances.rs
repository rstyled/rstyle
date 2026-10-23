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
    if owner.instance.is_some()
        || owner.reference_instance.is_some()
        || owner.reference_forwarder.is_some()
        || !owner.dependencies.is_empty()
    {
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
        let instance = self
            .instance
            .as_ref()
            .ok_or("Store is not a scalar instance")?;
        let (owner, ordinal, _) = self
            .scalar_instance_origin()
            .ok_or("Missing scalar instance origin")?;
        let original = owner
            .functions
            .get(ordinal)
            .ok_or("Missing scalar instance origin")?;
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

        let export = Export {
            schema: SCHEMA,
            owner_namespace: &owner.namespace,
            definition_ordinal: ordinal,
            projection,
        };
        serde_json::to_writer(writer, &export).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests;
