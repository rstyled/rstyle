//! Generic two-child vocabulary; the initial support boundary remains narrow.
use super::*;

pub(super) fn return_error(tree: &TypeTree) -> Option<&TypeTree> {
    match tree {
        TypeTree::Result { ok, err } if matches!(ok.as_ref(), TypeTree::Scalar(unit) if unit == "unit") => {
            Some(err)
        }
        TypeTree::Struct { fields, .. }
            if fields.len() == 1
                && fields[0].name == "0"
                && matches!(&fields[0].ty, TypeTree::Scalar(unit) if unit == "unit") =>
        {
            Some(tree)
        }
        _ => None,
    }
}

pub(super) fn checked_error(tree: &TypeTree, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let TypeTree::Struct {
        nominal, fields, ..
    } = tree
    else {
        return Err("Fresh Result error requires an exact local unit-field struct".into());
    };
    if fields.len() != 1
        || fields[0].name != "0"
        || !matches!(&fields[0].ty, TypeTree::Scalar(unit) if unit == "unit")
        || !facts.nominal_resolution()?.checked_no_drop(nominal)
    {
        return Err("Fresh Result error lacks its separately checked no-drop nominal proof".into());
    }
    Ok(())
}

pub(crate) fn payload(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(TypeToken, TypeToken), String> {
    let TypeToken::Result { ok, err } = crate::data_aggregates::parsed_type(node)? else {
        return Err("Result construction requires its concrete two-child type".into());
    };
    if !matches!(ok.as_ref(), TypeToken::Scalar(unit) if unit == "unit") {
        return Err("Initial concrete Result supports only a unit success payload".into());
    }

    let TypeToken::Struct { nominal, fields } = err.as_ref() else {
        return Err("Initial concrete Result requires a nominal unit-field error".into());
    };
    if fields != &[("0".into(), TypeToken::Scalar("unit".into()))]
        || !facts.nominal_resolution()?.checked_no_drop(nominal)
    {
        return Err("Result construction lacks exact no-drop error identity".into());
    }
    Ok((*ok, *err))
}

pub(super) fn contains_owned(ty: &TypeToken, error: &TypeToken) -> bool {
    if ty == error {
        return true;
    }
    match ty {
        TypeToken::Result { .. } => true,
        TypeToken::Tuple(values) => values.iter().any(|ty| contains_owned(ty, error)),
        TypeToken::Shared(ty)
        | TypeToken::Slice(ty)
        | TypeToken::Option(ty)
        | TypeToken::Array { element: ty, .. } => contains_owned(ty, error),
        TypeToken::Struct { fields, .. } => fields.iter().any(|(_, ty)| contains_owned(ty, error)),
        _ => false,
    }
}

pub(super) fn used(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    for ty in facts
        .signature
        .params
        .iter()
        .map(|parameter| &parameter.ty)
        .chain([&facts.signature.return_ty])
        .chain(facts.bindings.iter().map(|binding| &binding.ty))
    {
        if used_type(ty, facts)? {
            return Ok(true);
        }
    }
    used_node(&facts.body, facts)
}

fn used_node(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if let Some(ty) = &node.ty
        && used_type(ty, facts)?
    {
        return Ok(true);
    }
    for child in &node.children {
        if used_node(child, facts)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn used_type(ty: &crate::ResolvedTy, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(text) = &ty.normalized else {
        return Ok(false);
    };
    if !text.contains("\"kind\":\"result\"") && !text.contains("\"kind\":\"struct\"") {
        return Ok(false);
    }
    used_token(&crate::data_types::parse(text)?, facts)
}

fn used_token(token: &TypeToken, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    match token {
        TypeToken::Result { .. } => Ok(true),
        TypeToken::Struct { nominal, fields }
            if fields == &[("0".into(), TypeToken::Scalar("unit".into()))] =>
        {
            Ok(facts.nominal_resolution()?.checked_no_drop(nominal))
        }
        TypeToken::Tuple(values) => used_children(values.iter(), facts),
        TypeToken::Struct { fields, .. } => used_children(fields.iter().map(|(_, ty)| ty), facts),
        TypeToken::Array { element, .. } | TypeToken::Option(element) => used_token(element, facts),
        // Borrowed owner permission remains separate. No borrowed Result was
        // previously supported, so it cannot acquire fresh-result permission.
        TypeToken::Shared(element) | TypeToken::Slice(element) => Ok(contains_result(element)),
        _ => Ok(false),
    }
}

fn used_children<'a>(
    children: impl Iterator<Item = &'a TypeToken>,
    facts: &dyn EvidenceFacts,
) -> Result<bool, String> {
    for child in children {
        if used_token(child, facts)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn contains_result(token: &TypeToken) -> bool {
    match token {
        TypeToken::Result { .. } => true,
        TypeToken::Tuple(values) => values.iter().any(contains_result),
        TypeToken::Struct { fields, .. } => fields.iter().any(|(_, ty)| contains_result(ty)),
        TypeToken::Array { element, .. }
        | TypeToken::Shared(element)
        | TypeToken::Slice(element)
        | TypeToken::Option(element) => contains_result(element),
        _ => false,
    }
}
