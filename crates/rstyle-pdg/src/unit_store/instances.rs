//! Separate instance ownership; definition ordinals and ordinary rosters stay intact.
use super::*;
use crate::scalar_instances::{CheckedProjection, Projection, ScalarArgument};

pub(super) struct InstanceRecord {
    origin: OriginalInstance<ScalarArgument>,
    pub(super) checked: CheckedProjection,
}

pub(super) struct OriginalInstance<A> {
    owner: Arc<UnitStore>,
    definition: FunctionId,
    argument: A,
}

impl<A: Copy> OriginalInstance<A> {
    pub(super) fn get(&self) -> (&Arc<UnitStore>, usize, A) {
        (&self.owner, self.definition.0 as usize, self.argument)
    }
}

pub(super) trait InstanceProof {
    type Checked;
    type Argument: Copy;
    const LABEL: &'static str;

    fn verify(&self, original: &HirFnFacts) -> Result<Self::Checked, String>;
    fn facts(checked: &Self::Checked) -> &HirFnFacts;
    fn argument(checked: &Self::Checked) -> Self::Argument;
    fn install(
        self,
        store: &mut UnitStore,
        origin: OriginalInstance<Self::Argument>,
        checked: Self::Checked,
    ) -> Result<(), String>;
}

pub(super) fn capture_instance<P: InstanceProof>(
    owner: Arc<UnitStore>,
    definition: usize,
    proof: P,
) -> Result<UnitStore, String> {
    if owner.has_instance() {
        return Err(format!(
            "{} requires an original definition owner",
            P::LABEL
        ));
    }
    let original = owner
        .functions
        .get(definition)
        .ok_or_else(|| format!("{} names an absent original definition", P::LABEL))?;
    let checked = proof.verify(original)?;

    let mut builder = Builder::new(vec![P::facts(&checked).clone()])?;
    if builder.store.functions[0] != *P::facts(&checked) {
        return Err(format!(
            "{} normalization changed its checked evidence",
            P::LABEL
        ));
    }
    let origin = OriginalInstance {
        owner,
        definition: local_id(definition, FunctionId)?,
        argument: P::argument(&checked),
    };
    proof.install(&mut builder.store, origin, checked)?;
    builder.dependencies = dependency_analyses(&builder.store.dependencies);
    builder.capture()
}

impl InstanceProof for &Projection {
    type Checked = CheckedProjection;
    type Argument = ScalarArgument;
    const LABEL: &'static str = "Scalar instance";

    fn verify(&self, original: &HirFnFacts) -> Result<CheckedProjection, String> {
        let checked = crate::scalar_instances::verify(original, self)?;
        crate::scalar_instances::verify_owner(original, &checked)?;
        Ok(checked)
    }

    fn facts(checked: &CheckedProjection) -> &HirFnFacts {
        checked.facts()
    }

    fn argument(checked: &CheckedProjection) -> ScalarArgument {
        checked.scalar_argument()
    }

    fn install(
        self,
        store: &mut UnitStore,
        origin: OriginalInstance<ScalarArgument>,
        checked: CheckedProjection,
    ) -> Result<(), String> {
        store.instance = Some(InstanceRecord { origin, checked });
        Ok(())
    }
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
        capture_instance(owner, definition, projection)
    }

    /// Exact source owner and ordered structural argument; neither a namespace
    /// nor a display string substitutes for the original compiler definition.
    pub fn scalar_instance_origin(&self) -> Option<(&Arc<UnitStore>, usize, ScalarArgument)> {
        self.instance.as_ref().map(|instance| instance.origin.get())
    }
}

/// A retained projection always binds to one immutable original owner and ordinal.
pub(super) trait RetainedInstance: Sized {
    type Argument: Copy + PartialEq;
    const STORE_ERROR: &'static str;
    const ORIGINAL_ERROR: &'static str;
    fn select(store: &UnitStore) -> Option<&Self>;
    fn origin(&self) -> (&Arc<UnitStore>, usize, Self::Argument);
}

pub(super) trait ConstructorProjection {
    type Argument: PartialEq;
    fn facts(&self) -> &HirFnFacts;
    fn argument(&self) -> Self::Argument;
}

pub(super) trait RetainedConstructor: RetainedInstance {
    type Checked: ConstructorProjection<Argument = Self::Argument>;
    const RETAINED_ERROR: &'static str;
    fn observation(&self) -> &serde_json::Value;
    fn checked(&self) -> &Self::Checked;
    fn verify_original(&self, original: &HirFnFacts) -> Result<Self::Checked, String>;

    fn verify_store(&self, _store: &UnitStore) -> Result<(), String> {
        Ok(())
    }

    fn reverify(&self, original: &HirFnFacts, functions: &[HirFnFacts]) -> Result<(), String> {
        let checked = self.verify_original(original)?;
        if checked.facts() != self.checked().facts()
            || checked.argument() != self.checked().argument()
            || functions != std::slice::from_ref(checked.facts())
        {
            return Err(Self::RETAINED_ERROR.into());
        }
        Ok(())
    }
}

pub(super) struct RetainedInput<'a, R: RetainedInstance> {
    pub record: &'a R,
    pub owner: &'a Arc<UnitStore>,
    pub ordinal: usize,
    pub argument: R::Argument,
    pub original: &'a HirFnFacts,
}

pub(super) fn retained_input<R: RetainedInstance>(
    store: &UnitStore,
) -> Result<RetainedInput<'_, R>, String> {
    let record = R::select(store).ok_or(R::STORE_ERROR)?;
    let (owner, ordinal, argument) = record.origin();
    let original = owner.functions.get(ordinal).ok_or(R::ORIGINAL_ERROR)?;
    Ok(RetainedInput {
        record,
        owner,
        ordinal,
        argument,
        original,
    })
}

impl RetainedInstance for InstanceRecord {
    type Argument = ScalarArgument;
    const STORE_ERROR: &'static str = "Store is not a scalar instance";
    const ORIGINAL_ERROR: &'static str = "Missing scalar instance origin";

    fn select(store: &UnitStore) -> Option<&Self> {
        store.instance.as_ref()
    }

    fn origin(&self) -> (&Arc<UnitStore>, usize, ScalarArgument) {
        self.origin.get()
    }
}
