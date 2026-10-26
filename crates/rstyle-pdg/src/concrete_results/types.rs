//! Generic two-child vocabulary; the initial support boundary remains narrow.
use super::*;

pub(super) fn return_error(tree: &TypeTree) -> Option<&TypeTree> {
    match tree {
        TypeTree::Result { ok, err }
            if scalar_errors::GRAMMAR
                .permits(&TypeToken::from_tree(ok), &TypeToken::from_tree(err)) =>
        {
            Some(err)
        }
        TypeTree::Result { ok, err }
            if copy_errors::GRAMMAR
                .permits(&TypeToken::from_tree(ok), &TypeToken::from_tree(err)) =>
        {
            Some(err)
        }
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
        TypeTree::Result { ok, err } if crate::borrowed_result_types::tree_result(ok, err) => {
            Some(err)
        }
        TypeTree::MixedEnum { .. }
            if crate::borrowed_result_types::error(&TypeToken::from_tree(tree)).is_some() =>
        {
            Some(tree)
        }
        _ => None,
    }
}

pub(super) fn checked_error(tree: &TypeTree, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let token = TypeToken::from_tree(tree);
    if copy_errors::error(&token).is_some() {
        return copy_errors::checked_error(&token, facts);
    }
    if scalar_errors::error(&token).is_some() {
        return scalar_errors::checked_error(&token, facts);
    }
    if let Some(nominal) = crate::borrowed_result_types::error(&token) {
        if !crate::extraction_profile::facts_support(
            facts,
            crate::borrowed_result_types::CAPABILITY,
        ) || !facts.nominal_resolution()?.checked_no_drop(nominal)
        {
            return Err(
                "Borrowed Result error lacks its capability or independent no-drop proof".into(),
            );
        }
        return Ok(());
    }
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
    if let Some(error) = crate::named_error_payloads::error_tree(facts)? {
        if ok.as_ref() != &TypeToken::Scalar("usize".into())
            || err.as_ref() != &TypeToken::from_tree(&error)
        {
            return Err("Named error Result differs from its exact checked return boundary".into());
        }
        return Ok((*ok, *err));
    }
    if crate::copy_tagged_values::candidate(facts) {
        verify_conversion_payload(&ok, &err, facts)?;
        return Ok((*ok, *err));
    }
    if fresh_products::GRAMMAR.permits(&ok, &err) {
        let boundary = fresh_products::product_boundary(facts)?;
        if *ok != boundary.ok || *err != boundary.error {
            return Err("Fresh product constructor differs from its whole return boundary".into());
        }
        return Ok((*ok, *err));
    }
    if scalar_errors::GRAMMAR.permits(&ok, &err) {
        scalar_errors::result_boundary(facts)?.verify_payload(&ok, &err)?;
        return Ok((*ok, *err));
    }
    if copy_errors::GRAMMAR.permits(&ok, &err) {
        copy_errors::checked_error(&err, facts)?;
        return Ok((*ok, *err));
    }
    if crate::borrowed_result_types::GRAMMAR.permits(&ok, &err) {
        if !crate::extraction_profile::facts_support(
            facts,
            crate::borrowed_result_types::CAPABILITY,
        ) || !facts.nominal_resolution()?.checked_no_drop(
            crate::borrowed_result_types::error(&err).expect("checked error grammar"),
        ) {
            return Err(
                "Borrowed Result constructor lacks its capability or no-drop authority".into(),
            );
        }
        return Ok((*ok, *err));
    }
    verify_unit_payload(&ok, &err, facts)?;

    Ok((*ok, *err))
}

fn verify_unit_payload(
    ok: &TypeToken,
    err: &TypeToken,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    if !matches!(ok, TypeToken::Scalar(unit) if unit == "unit") {
        return Err("Initial concrete Result supports only a unit success payload".into());
    }

    let TypeToken::Struct { nominal, fields } = err else {
        return Err("Initial concrete Result requires a nominal unit-field error".into());
    };
    if fields != &[("0".into(), TypeToken::Scalar("unit".into()))]
        || !facts.nominal_resolution()?.checked_no_drop(nominal)
    {
        return Err("Result construction lacks exact no-drop error identity".into());
    }

    Ok(())
}

fn verify_conversion_payload(
    ok: &TypeToken,
    err: &TypeToken,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let resolution = facts.nominal_resolution()?;
    let receipt = resolution
        .copy_conversion_source(facts.facts())?
        .ok_or("Missing exact conversion Result proof")?;
    if (TypeToken::Result {
        ok: Box::new(ok.clone()),
        err: Box::new(err.clone()),
    }) != TypeToken::from_tree(receipt.output())
    {
        return Err("Conversion Result differs from its exact checked output".into());
    }
    Ok(())
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
        TypeToken::Enum { variants, .. } => {
            variants.iter().any(|(_, ty)| contains_owned(ty, error))
        }
        TypeToken::MixedEnum { variants, .. } => variants
            .iter()
            .flat_map(|(_, shape)| shape.fields())
            .any(|ty| contains_owned(ty, error)),
        _ => false,
    }
}

pub(super) fn used(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    usage(facts, false)
}

pub(super) fn scalar_used(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    usage(facts, true)
}

fn usage(facts: &dyn EvidenceFacts, scalar_only: bool) -> Result<bool, String> {
    for ty in facts
        .signature
        .params
        .iter()
        .map(|parameter| &parameter.ty)
        .chain([&facts.signature.return_ty])
        .chain(facts.bindings.iter().map(|binding| &binding.ty))
    {
        if used_type(ty, facts, scalar_only)? {
            return Ok(true);
        }
    }
    used_node(&facts.body, facts, scalar_only)
}

fn used_node(node: &HirNode, facts: &dyn EvidenceFacts, scalar_only: bool) -> Result<bool, String> {
    if let Some(ty) = &node.ty
        && used_type(ty, facts, scalar_only)?
    {
        return Ok(true);
    }
    for child in &node.children {
        if used_node(child, facts, scalar_only)? {
            return Ok(true);
        }
    }

    Ok(false)
}

fn used_type(
    ty: &crate::ResolvedTy,
    facts: &dyn EvidenceFacts,
    scalar_only: bool,
) -> Result<bool, String> {
    let Some(text) = &ty.normalized else {
        return Ok(false);
    };
    if !text.contains("\"kind\":\"result\"")
        && !text.contains("\"kind\":\"struct\"")
        && !text.contains("\"kind\":\"mixed_enum\"")
    {
        return Ok(false);
    }
    used_token(&crate::data_types::parse(text)?, facts, scalar_only)
}

fn used_token(
    token: &TypeToken,
    facts: &dyn EvidenceFacts,
    scalar_only: bool,
) -> Result<bool, String> {
    if scalar_errors::owned_error(token, facts)? {
        return Ok(true);
    }

    match token {
        TypeToken::Result { .. } => Ok(!scalar_only || scalar_errors::contains_result(token)),
        TypeToken::MixedEnum { .. } if crate::borrowed_result_types::error(token).is_some() => {
            Ok(!scalar_only)
        }
        TypeToken::Struct { nominal, fields }
            if fields == &[("0".into(), TypeToken::Scalar("unit".into()))] =>
        {
            Ok(!scalar_only && facts.nominal_resolution()?.checked_no_drop(nominal))
        }
        TypeToken::Tuple(values) => used_children(values.iter(), facts, scalar_only),
        TypeToken::Struct { fields, .. } => {
            used_children(fields.iter().map(|(_, ty)| ty), facts, scalar_only)
        }
        TypeToken::Enum { variants, .. } => {
            used_children(variants.iter().map(|(_, ty)| ty), facts, scalar_only)
        }
        TypeToken::MixedEnum { variants, .. } => used_children(
            variants.iter().flat_map(|(_, shape)| shape.fields()),
            facts,
            scalar_only,
        ),
        TypeToken::Array { element, .. } | TypeToken::Option(element) => {
            used_token(element, facts, scalar_only)
        }
        // Borrowed owner permission remains separate. No borrowed Result was
        // previously supported, so it cannot acquire fresh-result permission.
        TypeToken::Shared(element) | TypeToken::Slice(element) => Ok(if scalar_only {
            scalar_errors::contains_result(element)
        } else {
            contains_result(element)
        }),
        _ => Ok(false),
    }
}

fn used_children<'a>(
    children: impl Iterator<Item = &'a TypeToken>,
    facts: &dyn EvidenceFacts,
    scalar_only: bool,
) -> Result<bool, String> {
    for child in children {
        if used_token(child, facts, scalar_only)? {
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
        TypeToken::Enum { variants, .. } => variants.iter().any(|(_, ty)| contains_result(ty)),
        TypeToken::MixedEnum { variants, .. } => variants
            .iter()
            .flat_map(|(_, shape)| shape.fields())
            .any(contains_result),
        TypeToken::Array { element, .. }
        | TypeToken::Shared(element)
        | TypeToken::Slice(element)
        | TypeToken::Option(element) => contains_result(element),
        _ => false,
    }
}
