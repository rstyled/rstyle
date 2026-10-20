//! Bounded array-field views reuse existing helper reads and reference outputs.

use crate::{
    data_origins::Origin, data_types::TypeToken, field_locations::Projection,
    unit_store::EvidenceFacts,
};

pub(crate) const SUMMARY: &str = "rstyle.helper.borrowed-array-view/1";

pub(crate) fn verify_origin(
    origin: &Origin,
    facts: &dyn EvidenceFacts,
    result: &TypeToken,
) -> Result<(), String> {
    if !crate::extraction_profile::facts_support(facts, crate::data_subslices::ARRAY_CAPABILITY)
        || origin.path.first() != Some(&Projection::SharedReferent)
        || origin.path.len() < 2
        || !origin.path[1..]
            .iter()
            .all(|part| matches!(part, Projection::StructField { .. }))
    {
        return Err("Array view helper requires a registered shared formal-field origin".into());
    }
    let TypeToken::Shared(referent) = result else {
        return Err("Array view helper requires a shared slice result".into());
    };
    let TypeToken::Slice(element) = referent.as_ref() else {
        return Err("Array view helper requires a shared slice result".into());
    };
    match crate::helper_reads::type_at(facts, origin.parameter, &origin.path)? {
        TypeToken::Array {
            element: actual, ..
        } if actual == *element && matches!(element.as_ref(), TypeToken::Scalar(_)) => Ok(()),
        _ => Err("Array view helper result disagrees with its formal array element".into()),
    }
}

pub(crate) fn projected(facts: &dyn EvidenceFacts, outputs: &Option<Vec<Origin>>) -> bool {
    let Some(outputs) = outputs.as_ref().filter(|outputs| !outputs.is_empty()) else {
        return false;
    };
    let Some(result) = facts
        .signature
        .return_ty
        .normalized
        .as_deref()
        .and_then(|text| crate::data_types::parse(text).ok())
    else {
        return false;
    };
    outputs
        .iter()
        .all(|origin| verify_origin(origin, facts, &result).is_ok())
}

pub(crate) fn array_operation(node: &crate::PdgNode) -> bool {
    node.attributes.construct.as_deref() == Some("data.subslice-place")
        && node
            .annotations
            .get(crate::data_subslices::BOUNDS_KEY)
            .and_then(|bounds| bounds.get("array_length"))
            .and_then(serde_json::Value::as_u64)
            .is_some()
}
