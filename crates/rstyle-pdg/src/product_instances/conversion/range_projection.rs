//! A real Range-to-Span conversion retains the checked original local From body.
use super::*;
use crate::concrete_types::{InterfaceObservations, Observation as TypeObservation};

pub(in super::super) fn verify(
    canonical: &HirFnFacts,
    instance: &Value,
    span: &TypeTree,
    local: &HirFnFacts,
) -> Result<CheckedConversion, String> {
    let conversions = instance["conversions"]
        .as_array()
        .ok_or("Missing Range conversion roster")?;
    let [conversion] = conversions.as_slice() else {
        return Err("Range instance requires exactly one complete Into/From conversion".into());
    };
    let observed: Observation =
        serde_json::from_value(conversion.clone()).map_err(|e| e.to_string())?;

    let (call, receiver) = source::site(canonical)?;
    operation(&observed, call, receiver)?;
    let input = local_input(local, span)?;

    let span_identity = source::span_identity(canonical, &observed, span)?;
    let range_identity = source::range_identity(canonical, &observed, &input)?;
    result_type(call, span)?;

    model::verify_range(&observed, &span_identity, &range_identity)?;
    authority::verify_range(canonical, &observed, call)?;

    source::join_local_from(
        canonical,
        local,
        &instance["selected_local_from"],
        &observed.from,
    )?;

    Ok(CheckedConversion {
        call: call.clone(),
        receiver: receiver.clone(),
        input,
    })
}

fn local_input(local: &HirFnFacts, span: &TypeTree) -> Result<TypeTree, String> {
    let interface = local
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    interface.validate()?;
    let [
        TypeObservation::Captured {
            ty: input @ TypeTree::RangeUsize { .. },
        },
    ] = interface.parameters.as_slice()
    else {
        return Err("Selected local From lacks its actual complete owned Range interface".into());
    };

    if interface.result != (TypeObservation::Captured { ty: span.clone() }) {
        return Err("Selected local From result differs from the entire captured Span".into());
    }

    let (_, owner_input) = crate::nominal_boundary::resolution::range_from_owner(local)?
        .ok_or("Selected local Range From lacks its complete lexical ownership proof")?;
    if owner_input != *input {
        return Err("Selected local From owner and captured Range input disagree".into());
    }

    crate::range_values::consuming::verify_fields(local)?;
    Ok(input.clone())
}

#[cfg(test)]
mod tests;
