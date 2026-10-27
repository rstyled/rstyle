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
    owner: &'a UnitStore,
    observation: &'a Value,
    argument: SpanArgument,
}

impl InstanceProof for ConstructorProof<'_> {
    type Checked = CheckedProjection;
    type Argument = SpanArgument;
    const LABEL: &'static str = "Product constructor";

    fn verify(&self, original: &HirFnFacts) -> Result<CheckedProjection, String> {
        verify_projection(self.owner, original, self.observation, self.argument)
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
    ) -> Result<(), String> {
        install_dependency(store, &origin, &checked, self.observation)?;
        store.product_instance = Some(ProductInstanceRecord {
            origin,
            checked,
            observation: self.observation.clone(),
        });
        Ok(())
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

    fn verify_store(&self, store: &UnitStore) -> Result<(), String> {
        verify_dependencies(store, self)
    }

    fn verify_original(&self, original: &HirFnFacts) -> Result<CheckedProjection, String> {
        let (owner, _, argument) = self.origin.get();
        verify_projection(owner, original, &self.observation, argument)
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
            Arc::clone(&owner),
            definition,
            ConstructorProof {
                owner: &owner,
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

fn selected_local<'a>(
    owner: &'a UnitStore,
    observation: &Value,
) -> Result<(usize, &'a HirFnFacts), String> {
    let identity = observation["selected_local_from"]["definition"]["definition_identity"]
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or("Product Range instance lacks its selected local From identity")?;
    let mut candidates = owner.functions.iter().enumerate().filter(|(_, facts)| {
        facts
            .build
            .frontend_options
            .get("definition_identity")
            .and_then(Value::as_str)
            == Some(identity)
    });

    let selected = candidates
        .next()
        .ok_or("Selected local From is absent from the original owner")?;
    if candidates.next().is_some() {
        return Err("Selected local From is ambiguous in the original owner".into());
    }

    if owner.pair(selected.0).is_none()
        || owner.certified.get(selected.0).is_none_or(Option::is_none)
    {
        return Err("Selected local From lacks original complete certification".into());
    }

    Ok(selected)
}

fn verify_projection(
    owner: &UnitStore,
    original: &HirFnFacts,
    observation: &Value,
    argument: SpanArgument,
) -> Result<CheckedProjection, String> {
    match argument {
        SpanArgument::Span => {
            crate::product_instances::verify_constructor_projection(original, observation, argument)
        }
        SpanArgument::RangeUsize => {
            let (_, local) = selected_local(owner, observation)?;
            crate::product_instances::verify_range_constructor_projection(
                original,
                observation,
                local,
            )
        }
    }
}

fn install_dependency(
    store: &mut UnitStore,
    origin: &OriginalInstance<SpanArgument>,
    checked: &CheckedProjection,
    observation: &Value,
) -> Result<(), String> {
    let Some(local) = checked.local_from() else {
        return if checked.argument() == SpanArgument::Span {
            Ok(())
        } else {
            Err("Product Range projection lacks its checked local From".into())
        };
    };

    let (owner, _, argument) = origin.get();
    let (ordinal, selected) = selected_local(owner, observation)?;
    if argument != SpanArgument::RangeUsize || selected != local {
        return Err("Product dependency differs from its checked original local From".into());
    }

    store.external_links[0].insert(
        site(checked.operation()),
        ExternalTarget {
            dependency: 0,
            function: local_id(ordinal, FunctionId)?,
        },
    );
    store.dependencies.push(Arc::clone(owner));
    Ok(())
}

fn verify_dependencies(store: &UnitStore, record: &ProductInstanceRecord) -> Result<(), String> {
    let (owner, _, argument) = record.origin.get();
    if store.external_links.len() != 1 {
        return Err("Product instance has another external-link roster".into());
    }
    if argument == SpanArgument::Span {
        return if store.dependencies.is_empty()
            && store.external_links[0].is_empty()
            && record.checked.local_from().is_none()
        {
            Ok(())
        } else {
            Err("Span product instance has an unexpected dependency".into())
        };
    }

    let (ordinal, selected) = selected_local(owner, &record.observation)?;
    let expected = ExternalTarget {
        dependency: 0,
        function: local_id(ordinal, FunctionId)?,
    };
    if store.dependencies.len() != 1
        || !Arc::ptr_eq(&store.dependencies[0], owner)
        || store.external_links[0].len() != 1
        || !store.external_links[0]
            .get(&site(record.checked.operation()))
            .is_some_and(|actual| {
                actual.dependency == expected.dependency && actual.function == expected.function
            })
        || record.checked.local_from() != Some(selected)
    {
        return Err("Range product instance loses its exact original From dependency".into());
    }
    Ok(())
}
