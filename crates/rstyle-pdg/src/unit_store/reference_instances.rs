//! Original reference constructors retain their immutable source definition owner.
#[cfg(test)]
mod tests;

use super::instances::{
    ConstructorProjection, InstanceProof, OriginalInstance, RetainedConstructor, RetainedInstance,
    capture_instance,
};
use super::*;
use crate::reference_instances::{CheckedProjection, Haystack};
use serde_json::Value;

pub(super) struct ReferenceInstanceRecord {
    origin: OriginalInstance<Haystack>,
    pub(super) checked: CheckedProjection,
    observation: Value,
}

struct ConstructorProof<'a> {
    observation: &'a Value,
    haystack: Haystack,
}

impl InstanceProof for ConstructorProof<'_> {
    type Checked = CheckedProjection;
    type Argument = Haystack;
    const LABEL: &'static str = "Reference constructor";

    fn verify(&self, original: &HirFnFacts) -> Result<CheckedProjection, String> {
        crate::reference_instances::verify_constructor_projection(
            original,
            self.observation,
            self.haystack,
        )
    }

    fn facts(checked: &CheckedProjection) -> &HirFnFacts {
        checked.facts()
    }

    fn argument(checked: &CheckedProjection) -> Haystack {
        checked.haystack()
    }

    fn install(
        self,
        store: &mut UnitStore,
        origin: OriginalInstance<Haystack>,
        checked: CheckedProjection,
    ) {
        store.reference_instance = Some(ReferenceInstanceRecord {
            origin,
            checked,
            observation: self.observation.clone(),
        });
    }
}

impl ConstructorProjection for CheckedProjection {
    type Argument = Haystack;

    fn facts(&self) -> &HirFnFacts {
        CheckedProjection::facts(self)
    }

    fn argument(&self) -> Haystack {
        self.haystack()
    }
}

impl RetainedConstructor for ReferenceInstanceRecord {
    type Checked = CheckedProjection;
    const RETAINED_ERROR: &'static str =
        "Reference sidecar differs from its retained checked instance";

    fn observation(&self) -> &Value {
        &self.observation
    }

    fn checked(&self) -> &CheckedProjection {
        &self.checked
    }

    fn verify_original(&self, original: &HirFnFacts) -> Result<CheckedProjection, String> {
        crate::reference_instances::verify_constructor_projection(
            original,
            &self.observation,
            self.origin.get().2,
        )
    }
}

impl UnitStore {
    /// Create a separate one-function owner from the complete checked original
    /// constructor. This grants neither ordinary transport nor caller linkage.
    pub fn capture_reference_constructor(
        owner: Arc<Self>,
        definition: usize,
        observation: &Value,
        haystack: Haystack,
    ) -> Result<Self, String> {
        capture_instance(
            owner,
            definition,
            ConstructorProof {
                observation,
                haystack,
            },
        )
    }

    /// Exact original function ordinal and concrete structural substitution.
    pub fn reference_instance_origin(&self) -> Option<(&Arc<UnitStore>, usize, Haystack)> {
        self.reference_instance
            .as_ref()
            .map(|instance| instance.origin.get())
    }
}

impl RetainedInstance for ReferenceInstanceRecord {
    type Argument = Haystack;
    const STORE_ERROR: &'static str = "Store is not a reference constructor instance";
    const ORIGINAL_ERROR: &'static str = "Missing original constructor";

    fn select(store: &UnitStore) -> Option<&Self> {
        store.reference_instance.as_ref()
    }

    fn origin(&self) -> (&Arc<UnitStore>, usize, Haystack) {
        self.origin.get()
    }
}
