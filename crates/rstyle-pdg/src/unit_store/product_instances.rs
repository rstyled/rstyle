//! Original product constructors retain their immutable source definition owner.
use super::instances::{
    ConstructorProjection, InstanceProof, OriginalInstance, RetainedConstructor, RetainedInstance,
    capture_instance,
};
use super::*;
use crate::product_instances::{CheckedProjection, SpanArgument};
use serde_json::Value;

pub(super) struct ProductInstanceRecord {
    origin: OriginalInstance<SpanArgument>,
    pub(super) checked: CheckedProjection,
    observation: Value,
}

struct ConstructorProof<'a> {
    observation: &'a Value,
    argument: SpanArgument,
}

impl InstanceProof for ConstructorProof<'_> {
    type Checked = CheckedProjection;
    type Argument = SpanArgument;
    const LABEL: &'static str = "Product constructor";

    fn verify(&self, original: &HirFnFacts) -> Result<CheckedProjection, String> {
        crate::product_instances::verify_constructor_projection(
            original,
            self.observation,
            self.argument,
        )
    }

    fn facts(checked: &CheckedProjection) -> &HirFnFacts {
        checked.facts()
    }

    fn argument(checked: &CheckedProjection) -> SpanArgument {
        checked.argument()
    }

    fn install(
        self,
        store: &mut UnitStore,
        origin: OriginalInstance<SpanArgument>,
        checked: CheckedProjection,
    ) {
        store.product_instance = Some(ProductInstanceRecord {
            origin,
            checked,
            observation: self.observation.clone(),
        });
    }
}

impl ConstructorProjection for CheckedProjection {
    type Argument = SpanArgument;

    fn facts(&self) -> &HirFnFacts {
        CheckedProjection::facts(self)
    }

    fn argument(&self) -> SpanArgument {
        self.argument()
    }
}

impl RetainedConstructor for ProductInstanceRecord {
    type Checked = CheckedProjection;
    const RETAINED_ERROR: &'static str =
        "Product sidecar differs from its retained checked instance";

    fn observation(&self) -> &Value {
        &self.observation
    }

    fn checked(&self) -> &CheckedProjection {
        &self.checked
    }

    fn verify_original(&self, original: &HirFnFacts) -> Result<CheckedProjection, String> {
        crate::product_instances::verify_constructor_projection(
            original,
            &self.observation,
            self.origin.get().2,
        )
    }
}

impl UnitStore {
    /// Create a separate one-function owner from the complete checked original
    /// constructor. This grants neither ordinary transport nor caller linkage.
    pub fn capture_product_constructor(
        owner: Arc<Self>,
        definition: usize,
        observation: &Value,
        argument: SpanArgument,
    ) -> Result<Self, String> {
        capture_instance(
            owner,
            definition,
            ConstructorProof {
                observation,
                argument,
            },
        )
    }

    /// Exact original function ordinal and concrete structural substitution.
    pub fn product_instance_origin(&self) -> Option<(&Arc<UnitStore>, usize, SpanArgument)> {
        self.product_instance
            .as_ref()
            .map(|instance| instance.origin.get())
    }
}

impl RetainedInstance for ProductInstanceRecord {
    type Argument = SpanArgument;
    const STORE_ERROR: &'static str = "Store is not a product constructor instance";
    const ORIGINAL_ERROR: &'static str = "Missing original constructor";

    fn select(store: &UnitStore) -> Option<&Self> {
        store.product_instance.as_ref()
    }

    fn origin(&self) -> (&Arc<UnitStore>, usize, SpanArgument) {
        self.origin.get()
    }
}
