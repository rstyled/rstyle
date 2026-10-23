//! Exact checked From owners retain their constructor and immutable original owner.
use super::*;
use crate::reference_instances::{CheckedForwarderProjection, Haystack, ReferenceObservation};
use serde_json::Value;

pub(super) struct ForwarderRecord {
    pub(super) checked: CheckedForwarderProjection,
    pub(super) owner: Arc<UnitStore>,
    pub(super) ordinal: usize,
    pub(super) observation: Value,
}

impl UnitStore {
    /// Construct the independently proved original forwarding body and its sole
    /// checked constructor dependency at exactly the same concrete substitution.
    pub fn capture_reference_forwarder(
        owner: Arc<Self>,
        ordinal: usize,
        observation: &Value,
        constructor: Arc<Self>,
    ) -> Result<Self, String> {
        owner.require_original_forwarder_owner()?;
        let (constructor_owner, constructor_ordinal, haystack) = constructor
            .reference_instance_origin()
            .ok_or("Reference forwarder requires a checked constructor dependency")?;
        if !Arc::ptr_eq(&owner, constructor_owner) {
            return Err("Reference forwarder constructor belongs to another original owner".into());
        }

        let checked = crate::reference_instances::verify_forwarder_projection(
            ReferenceObservation {
                canonical: owner
                    .functions
                    .get(ordinal)
                    .ok_or("Absent original From definition")?,
                observation,
            },
            ReferenceObservation {
                canonical: &owner.functions[constructor_ordinal],
                observation: constructor
                    .reference_instance
                    .as_ref()
                    .ok_or("Missing checked constructor")?
                    .observation(),
            },
            haystack,
        )?;
        if constructor.functions.as_slice() != std::slice::from_ref(checked.constructor().facts())
            || constructor.certified.as_slice() != [Some(HelperId(0))]
        {
            return Err(
                "Reference forwarder dependency differs from its proved constructor".into(),
            );
        }

        let record = ForwarderRecord {
            checked,
            owner,
            ordinal,
            observation: observation.clone(),
        };
        Self::capture_checked_forwarder(record, constructor)
    }

    fn require_original_forwarder_owner(&self) -> Result<(), String> {
        if self.instance.is_some()
            || self.reference_instance.is_some()
            || self.reference_forwarder.is_some()
            || !self.dependencies.is_empty()
        {
            return Err("Reference forwarder requires its original unlinked owner".into());
        }
        Ok(())
    }

    fn capture_checked_forwarder(
        record: ForwarderRecord,
        constructor: Arc<Self>,
    ) -> Result<Self, String> {
        let checked = &record.checked;
        let builder = Builder::new(vec![checked.facts().clone()])?;
        if builder.store.functions[0] != *checked.facts() {
            return Err("Forwarder normalization changed checked evidence".into());
        }
        let mut builder = builder.with_reference_dependency(checked.call(), constructor);
        builder.store.reference_forwarder = Some(record);

        builder.capture()
    }

    /// Original definition and exact checked constructor substitution.
    pub fn reference_forwarder_origin(&self) -> Option<(&Arc<UnitStore>, usize, Haystack)> {
        self.reference_forwarder.as_ref().map(|record| {
            (
                &record.owner,
                record.ordinal,
                record.checked.constructor().haystack(),
            )
        })
    }
}

impl Builder {
    fn with_reference_dependency(mut self, call: &HirNode, constructor: Arc<UnitStore>) -> Self {
        self.store.external_links[0].insert(
            site(call),
            ExternalTarget {
                dependency: 0,
                function: FunctionId(0),
            },
        );
        self.store.dependencies.push(constructor);
        self.dependencies = dependency_analyses(&self.store.dependencies);

        self
    }
}
