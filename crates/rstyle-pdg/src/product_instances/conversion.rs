//! Checked full-value Into→From identity; never an origin-only or name-based summary.
mod authority;
mod model;
mod range_projection;
mod review;
pub(super) use range_projection::verify as verify_range;
mod source;
#[cfg(test)]
mod tests;

use super::SpanArgument;
use crate::{HirFnFacts, HirNode, concrete_types::TypeTree};
use model::Observation;
use serde_json::Value;

/// Nominal paths are compiler structural fields, joined to independently resolved
/// complete source trees; display values and opaque fingerprints grant no authority.
pub(super) fn verify_local_argument(
    canonical: &HirFnFacts,
    observed: &Value,
    original: &Value,
    tree: &TypeTree,
) -> Result<(), String> {
    let values = observed
        .as_array()
        .ok_or("Missing concrete product type argument")?;
    let [value] = values.as_slice() else {
        return Err("Concrete product type argument must be singular".into());
    };
    let value: model::Argument =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    let original: model::Structural =
        serde_json::from_value(original.clone()).map_err(|error| error.to_string())?;

    value.verify(&original)?;
    source::local_nominal(canonical, &original, tree)
}

/// Privately minted only after the complete selected forwarding/identity proof.
/// Source ownership, Copy/no-drop and instance activation remain separate proofs.
pub(super) struct CheckedConversion {
    call: HirNode,
    receiver: HirNode,
    input: TypeTree,
}

impl CheckedConversion {
    pub(super) fn input(&self) -> &TypeTree {
        &self.input
    }

    pub(super) fn call(&self) -> &HirNode {
        &self.call
    }
    pub(super) fn receiver(&self) -> &HirNode {
        &self.receiver
    }
}

pub(super) fn verify(
    canonical: &HirFnFacts,
    instance: &Value,
    span: &TypeTree,
    argument: SpanArgument,
) -> Result<CheckedConversion, String> {
    if argument != SpanArgument::Span {
        return Err(
            "Range conversion requires its separate consuming-callee and ownership proof".into(),
        );
    }
    let conversions = instance["conversions"]
        .as_array()
        .ok_or("Missing product conversion roster")?;
    let [conversion] = conversions.as_slice() else {
        return Err("Product instance requires exactly one complete Into/From conversion".into());
    };
    if instance.get("selected_local_from") != Some(&Value::Null) {
        return Err("Span identity conversion must select the reviewed reflexive core From".into());
    }
    let observation: Observation =
        serde_json::from_value(conversion.clone()).map_err(|error| error.to_string())?;
    let (call, receiver) = source::site(canonical)?;
    operation(&observation, call, receiver)?;

    let identity = source::span_identity(canonical, &observation, span)?;
    result_type(call, span)?;
    model::arguments(&observation.receiver_type, &identity, 1)?;
    model::arguments(&observation.result_type, &identity, 1)?;
    observation.into.structure(&identity, &[0, 1])?;
    observation.from.structure(&identity, &[0, 0])?;
    authority::verify(canonical, &observation, call)?;

    Ok(CheckedConversion {
        call: call.clone(),
        receiver: receiver.clone(),
        input: span.clone(),
    })
}

fn operation(observed: &Observation, call: &HirNode, receiver: &HirNode) -> Result<(), String> {
    if observed.identity != "rstyle.rust.product-into-from-observation/1"
        || observed.hir != call.hir
        || observed.span != call.span
        || observed.receiver != receiver.hir
        || !observed.adjustments.is_empty()
        || observed.from_selection_basis
            != "independent compiler selection of From<concrete input> for actual Span; not a core Into body/effect proof"
    {
        return Err(
            "Product conversion changes its original receiver/site or introduces adjustments"
                .into(),
        );
    }
    Ok(())
}

fn result_type(call: &HirNode, span: &TypeTree) -> Result<(), String> {
    let expected = crate::data_types::parse(&crate::data_types::token(span)?)?;
    crate::data_aggregates::verify_node_type(
        call,
        &expected,
        "Product conversion output differs from its entire captured Span value",
    )
}
