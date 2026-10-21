//! Stored slice results retain both the reference-slot read and referent origin.

use crate::{
    data_origins::Origin, data_types::TypeToken, field_locations::Projection, producer::PairView,
    unit_store::EvidenceFacts,
};

pub(crate) const CAPABILITY: &str = "rust.stored-slice-helper-results";
pub(crate) const SUMMARY: &str = "rstyle.helper.stored-slice-result/1";

pub(crate) fn verify_field(
    facts: &dyn EvidenceFacts,
    location: &crate::field_locations::Location,
) -> Result<(), String> {
    let crate::field_locations::Location::Captured {
        parameter,
        path,
        value,
    } = location
    else {
        return Ok(());
    };
    if !mixed_slot(facts, path, value)? {
        return Ok(());
    }

    let mut referent = path.clone();
    referent.push(Projection::SharedReferent);
    verify_origin(
        &Origin {
            parameter: *parameter,
            path: referent,
        },
        facts,
        &crate::data_types::parse(&crate::data_types::token(value)?)?,
    )
}

fn mixed_slot(
    facts: &dyn EvidenceFacts,
    path: &[Projection],
    value: &crate::concrete_types::TypeTree,
) -> Result<bool, String> {
    Ok(
        matches!(value, crate::concrete_types::TypeTree::Shared(inner)
            if matches!(inner.as_ref(), crate::concrete_types::TypeTree::Slice(_)))
            && path
                .iter()
                .any(|part| matches!(part, Projection::StructField { .. }))
            && facts.nominal_resolution()?.has_mixed_owners(),
    )
}

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    if crate::extraction_profile::policy_supports(profile, CAPABILITY) {
        return Ok(());
    }
    let fields = facts
        .build
        .frontend_options
        .data::<crate::field_locations::FieldObservations>()?;
    for field in &fields.operations {
        if let crate::field_locations::Location::Captured { path, value, .. } = &field.location
            && mixed_slot(facts, path, value)?
        {
            return Err("Stored slice slots require their registered document policy".into());
        }
    }
    Ok(())
}

pub(crate) fn verify_origin(
    origin: &Origin,
    facts: &dyn EvidenceFacts,
    result: &TypeToken,
) -> Result<(), String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || !stored_path(&origin.path)
        || !slice_result(result)
    {
        return Err(
            "Stored slice helper requires a registered shared formal-field referent".into(),
        );
    }
    let slot = &origin.path[..origin.path.len() - 1];
    if crate::helper_reads::type_at(facts, origin.parameter, slot)? != *result {
        return Err("Stored slice helper result disagrees with its reference slot".into());
    }
    Ok(())
}

fn slice_result(result: &TypeToken) -> bool {
    matches!(result, TypeToken::Shared(referent)
        if matches!(referent.as_ref(), TypeToken::Slice(element)
            if matches!(element.as_ref(), TypeToken::Scalar(value)
                if crate::concrete_types::scalar(value) && value != "unit")))
}

fn stored_path(path: &[Projection]) -> bool {
    path.len() >= 3
        && path.first() == Some(&Projection::SharedReferent)
        && path.last() == Some(&Projection::SharedReferent)
        && path[1..path.len() - 1]
            .iter()
            .all(|projection| matches!(projection, Projection::StructField { .. }))
}

pub(crate) fn projected(facts: &dyn EvidenceFacts, outputs: &Option<Vec<Origin>>) -> bool {
    outputs
        .as_ref()
        .is_some_and(|outputs| outputs.len() <= crate::concrete_types::MAX_ENTRIES)
        && crate::helper_references::projected_fields(facts, outputs, verify_origin)
}

pub(crate) fn verify_reads(
    facts: &dyn EvidenceFacts,
    pair: PairView<'_>,
    outputs: &Option<Vec<Origin>>,
) -> Result<(), String> {
    if !projected(facts, outputs) {
        return Err("Stored slice helper lacks verified field-reference outputs".into());
    }
    let reads = crate::helper_reads::derive(facts, pair)?
        .ok_or("Stored slice helper lacks independently verified slot reads")?;
    for origin in outputs.as_ref().into_iter().flatten() {
        let slot = &origin.path[..origin.path.len() - 1];
        let value = crate::helper_reads::type_at(facts, origin.parameter, slot)?;
        if !reads.iter().any(|read| {
            read.parameter == origin.parameter && read.path == slot && read.value == value
        }) {
            return Err("Stored slice helper output lacks its exact reference-slot read".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::slice_result;
    use crate::data_types::TypeToken;

    #[test]
    fn stored_results_require_shared_slices_of_supported_nonunit_scalars() {
        for (scalar, expected) in [
            ("u8", true),
            ("usize", true),
            ("unit", false),
            ("f64", false),
        ] {
            let result = TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(
                TypeToken::Scalar(scalar.into()),
            ))));
            assert_eq!(slice_result(&result), expected, "{scalar}");
        }
        assert!(!slice_result(&TypeToken::Slice(Box::new(
            TypeToken::Scalar("u8".into())
        ))));
        assert!(!slice_result(&TypeToken::Shared(Box::new(
            TypeToken::Scalar("u8".into())
        ))));
    }
}
