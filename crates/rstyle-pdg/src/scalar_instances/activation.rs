//! The private checked owner is the only bridge from generic source to scalar use.
use super::{CAPABILITY, ScalarArgument};
use crate::unit_store::EvidenceFacts;

#[cfg(test)]
mod tests;

/// Absence is ordinary frontend evidence, never an inferred instance.
pub(crate) fn verify(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = facts.scalar_instance() else {
        return Ok(false);
    };
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Checked scalar instance requires its registered capability".into());
    }
    if checked.facts() != facts.facts() {
        return Err("Checked scalar instance token differs from its owning facts".into());
    }

    interface(facts.facts(), checked.scalar_argument())?;
    Ok(true)
}

pub(crate) fn scalar(argument: ScalarArgument) -> &'static str {
    match argument {
        ScalarArgument::I32 => "i32",
        ScalarArgument::U32 => "u32",
    }
}

pub(crate) fn interface(facts: &crate::HirFnFacts, argument: ScalarArgument) -> Result<(), String> {
    let signature = &facts.signature;
    let [generic] = signature.generics.as_slice() else {
        return Err("Checked scalar helper requires one original type parameter".into());
    };
    let scalar = scalar(argument);

    if generic.name.is_empty()
        || signature.has_self
        || signature.receiver.is_some()
        || signature.is_async
        || signature.is_const
        || signature.is_unsafe
        || signature.is_variadic
        || signature.abi.value.as_deref() != Some("Rust")
        || signature.params.is_empty()
        || signature.params.iter().any(|parameter| {
            parameter.ty.normalized.as_deref() != Some(scalar)
                || parameter.ty.spelling != generic.name
        })
        || signature.return_ty.normalized.as_deref() != Some(scalar)
        || signature.return_ty.spelling != generic.name
    {
        return Err(
            "Checked scalar helper requires its original generic interface and concrete scalar types"
                .into(),
        );
    }
    Ok(())
}
