//! Exact bounded return vocabulary, after independent source/cleanup selection.
use super::*;

pub(crate) fn error_tree(facts: &dyn EvidenceFacts) -> Result<Option<TypeTree>, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Ok(None);
    }

    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;

    let Observation::Captured { ty } = &interface.result else {
        return Ok(None);
    };

    let error = match ty {
        TypeTree::Result { ok, err } if matches!(ok.as_ref(), TypeTree::Scalar(value) if value == "usize") => {
            err.as_ref()
        }
        error @ TypeTree::Struct { .. } => error,
        _ => return Ok(None),
    };

    let TypeTree::Struct {
        nominal, fields, ..
    } = &error
    else {
        return Ok(None);
    };

    if !matches!(fields.as_slice(), [field] if field.name == "0" && matches!(field.ty, TypeTree::MixedEnum { .. }))
    {
        return Ok(None);
    }

    if !facts.nominal_resolution()?.checked_no_drop(nominal) {
        return Err("Named error result lacks its independent whole cleanup proof".into());
    }

    Ok(Some(error.clone()))
}

pub(crate) fn permits_type(text: &str, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    // Most signatures have no nominal return. Avoid constructing source authority
    // for unrelated functions on the ordinary type-checking path.

    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || !facts
            .signature
            .return_ty
            .normalized
            .as_deref()
            .is_some_and(|text| text.contains("\"kind\":\"mixed_enum\""))
    {
        return Ok(false);
    }

    let Some(error) = error_tree(facts)? else {
        return Ok(false);
    };

    let token = crate::data_types::parse(text)?;

    let TypeTree::Struct { fields, .. } = &error else {
        unreachable!()
    };

    let permitted = token == TypeToken::from_tree(&error)
        || token == TypeToken::from_tree(&fields[0].ty)
        || token
            == (TypeToken::Result {
                ok: Box::new(TypeToken::Scalar("usize".into())),
                err: Box::new(TypeToken::from_tree(&error)),
            });

    Ok(permitted)
}
