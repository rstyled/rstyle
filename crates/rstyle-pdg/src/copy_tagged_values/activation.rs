//! Narrow complete-function authority; syntax alone never grants tagged values.
use super::*;
use crate::concrete_types::{InterfaceObservations, Observation};

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || facts
            .build
            .frontend_options
            .get(crate::tryfrom_calls::KEY)
            .is_none_or(|value| value["status"] != "observed")
    {
        return false;
    }
    let Ok(interface) = facts.build.frontend_options.data::<InterfaceObservations>() else {
        return false;
    };
    let [Observation::Captured { ty: input }] = interface.parameters.as_slice() else {
        return false;
    };
    matches!(
        (input, &interface.result),
        (
            TypeTree::Struct { .. },
            Observation::Captured {
                ty: TypeTree::MixedEnum { .. }
            }
        ) | (
            TypeTree::MixedEnum { .. },
            Observation::Captured {
                ty: TypeTree::Result { .. }
            }
        )
    )
}

pub(crate) fn body_operation(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    candidate(facts)
        && expression(facts).is_ok_and(|body| body.hir == node.hir && body.span == node.span)
}

pub(crate) fn permits_type(text: &str, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if !candidate(facts) {
        return Ok(false);
    }
    let resolution = facts.nominal_resolution()?;
    let receipt = resolution
        .copy_conversion_source(facts.facts())?
        .ok_or("Tagged values lack their exact checked original conversion")?;

    let token = crate::data_types::parse(text)?;
    Ok([
        Some(receipt.input()),
        Some(receipt.output()),
        Some(receipt.product()),
        receipt.error(),
    ]
    .into_iter()
    .flatten()
    .any(|tree| token == crate::data_types::TypeToken::from_tree(tree)))
}

pub(crate) fn verify_function(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if !candidate(facts) {
        return Ok(false);
    }
    let resolution = facts.nominal_resolution()?;
    let receipt = resolution
        .copy_conversion_source(facts.facts())?
        .ok_or("Missing original tagged conversion source/Copy/member proof")?;

    let contract = origins::derive_source_contract(facts, receipt)?;
    origins::validate_source_contract(&contract, facts, receipt)?;
    Ok(true)
}
