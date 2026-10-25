//! Whole-function evidence adapter; ordinary value/effect/graph checks remain required.
mod body;
mod integration;
pub(crate) use integration::{candidate, construct, operands, permits_type, source_findings};
#[cfg(test)]
mod tests;
mod variants;

use super::{Observation as Instance, constructors};
use crate::{
    HirNode,
    concrete_types::{InterfaceObservations, Observation, TypeTree},
    slice_patterns::semantics,
    unit_store::EvidenceFacts,
};

pub(crate) const VALUE_CAPABILITY: &str = "rust.checked-digit-product-values";

pub(crate) struct Checked<'a> {
    root: &'a HirNode,
    some: &'a HirNode,
    prefix: semantics::Checked<'a>,
    constructor: constructors::CheckedConstructor<'a>,
    instance: Instance,
}

impl<'a> Checked<'a> {
    pub(crate) fn root(&self) -> &'a HirNode {
        self.root
    }
    pub(crate) fn prefix(&self) -> &semantics::Checked<'a> {
        &self.prefix
    }
    pub(crate) fn constructor(&self) -> &constructors::CheckedConstructor<'a> {
        &self.constructor
    }
    pub(crate) fn instance(&self) -> &Instance {
        &self.instance
    }
    pub(crate) fn scalar(&self) -> &'a HirNode {
        self.constructor.fields()[1]
    }
    pub(crate) fn some(&self) -> &'a HirNode {
        self.some
    }
}

pub(crate) fn verify(input: &dyn EvidenceFacts) -> Result<Checked<'_>, String> {
    if !crate::extraction_profile::facts_support(input, VALUE_CAPABILITY) {
        return Err(
            "Digit product values require their registered whole-function capability".into(),
        );
    }
    let instance = instance(input)?;
    let tree = TypeTree::ScalarProductInstance(Box::new(instance.clone()));
    let resolution = input.nominal_resolution()?;
    let digits = resolution.checked_scalar_product_digits(input.facts(), &tree)?;
    let matched = body::verify(input)?;
    let prefix = semantics::verify(matched, input)?;
    if prefix.consumed() != digits {
        return Err("Digit product prefix length differs from its original concrete method".into());
    }
    let variant = variants::verify(&prefix, &instance, input)?;
    let context = constructors::Context {
        resolution: &resolution,
        instance: &instance,
    };
    let constructor = constructors::verify(variant.product, input, &context)?;
    if constructor.fields()[0] != prefix.remainder() {
        return Err("Digit product first field must retain the exact saved suffix value".into());
    }

    Ok(Checked {
        root: &input.body,
        some: variant.some,
        prefix,
        constructor,
        instance,
    })
}

fn instance(input: &dyn EvidenceFacts) -> Result<Instance, String> {
    let interface = input
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    interface.validate()?;
    let Observation::Captured {
        ty: TypeTree::Option(payload),
    } = &interface.result
    else {
        return Err("Digit product values require an exact captured Option result".into());
    };
    let TypeTree::ScalarProductInstance(instance) = payload.as_ref() else {
        return Err(
            "Digit product Option payload requires an exact scalar product instance".into(),
        );
    };

    instance.check_shape()?;
    Ok(instance.as_ref().clone())
}

/// Run a contextual action only after the complete independent function proof.
pub(crate) fn when_checked<T>(
    facts: &dyn EvidenceFacts,
    action: impl FnOnce(Checked<'_>) -> Result<T, String>,
) -> Result<Option<T>, String> {
    if !candidate(facts) {
        return Ok(None);
    }
    action(verify(facts)?).map(Some)
}
