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
        TypeTree::Result { ok, err } if success(&TypeToken::from_tree(ok), facts)? => err.as_ref(),
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

    let permitted = owned::permits_type(text, facts)?
        || token == TypeToken::from_tree(&error)
        || token == TypeToken::from_tree(&fields[0].ty)
        || matches!(&token, TypeToken::Result { ok, err }
            if success(ok, facts)? && err.as_ref() == &TypeToken::from_tree(&error));

    Ok(permitted)
}

/// Historical scalar permission remains independent of the reader extension.
fn success(token: &TypeToken, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if crate::native_reader_width::u16_success(token, facts) {
        return Ok(true);
    }
    if crate::extraction_profile::facts_support(facts, crate::widening_readers::CAPABILITY)
        && crate::widening_readers::success_shape(token)
    {
        return Ok(true);
    }

    if crate::id_reader_types::value(token, facts)? {
        return Ok(true);
    }

    if crate::id_reader_types::shape(token) {
        return crate::id_reader_types::success(token, facts);
    }

    if crate::reader_products::shape(token) {
        return crate::reader_products::success(token, facts);
    }

    Ok(
        matches!(token, TypeToken::Scalar(value) if value == "usize")
            || crate::extraction_profile::facts_support(
                facts,
                crate::named_error_result_propagation::CAPABILITY,
            ) && crate::named_error_result_propagation::success(token),
    )
}
