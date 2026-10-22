//! Separate instance ownership; definition ordinals and ordinary rosters stay intact.
use super::*;
use crate::scalar_instances::{CheckedProjection, Projection, ScalarArgument};

pub(super) struct InstanceRecord {
    owner: Arc<UnitStore>,
    definition: FunctionId,
    pub(super) checked: CheckedProjection,
}

impl UnitStore {
    /// Verify against an immutable original definition before constructing a
    /// separate one-body analysis owner. This is not a caller/artifact join.
    /// Ordinary bundle export deliberately rejects this store until original
    /// owner and projection replay have their own checked transport contract.
    pub fn capture_scalar_instance(
        owner: Arc<Self>,
        definition: usize,
        projection: &Projection,
    ) -> Result<Self, String> {
        if owner.instance.is_some() {
            return Err("Scalar instance requires an original definition owner".into());
        }

        let original = owner
            .functions
            .get(definition)
            .ok_or("Scalar instance names an absent original definition")?;
        let checked = crate::scalar_instances::verify(original, projection)?;
        crate::scalar_instances::verify_owner(original, &checked)?;

        let mut builder = Builder::new(vec![checked.facts().clone()])?;
        if builder.store.functions[0] != *checked.facts() {
            return Err("Scalar instance normalization changed its checked evidence".into());
        }

        builder.store.instance = Some(InstanceRecord {
            owner,
            definition: local_id(definition, FunctionId)?,
            checked,
        });
        builder.capture()
    }

    /// Exact source owner and ordered structural argument; neither a namespace
    /// nor a display string substitutes for the original compiler definition.
    pub fn scalar_instance_origin(&self) -> Option<(&Arc<UnitStore>, usize, ScalarArgument)> {
        let instance = self.instance.as_ref()?;
        Some((
            &instance.owner,
            instance.definition.0 as usize,
            instance.checked.scalar_argument(),
        ))
    }
}
