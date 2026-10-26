//! Fresh named variant and unary wrapper receipts.
use super::*;

/// Candidate selection is structural; no function or variant names confer authority.
pub(crate) fn constructor<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Option<Constructor<'a>>, String> {
    if node.kind != "expr_struct" || !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Ok(None);
    }

    if node.expansion.is_some() || node.is_unsafe_block {
        return Err("Fresh error construction requires complete ordinary source".into());
    }

    let token = crate::data_aggregates::parsed_type(node)?;
    match &token {
        TypeToken::MixedEnum { .. } => named(node, facts, token).map(Some),
        TypeToken::Struct { fields, .. } if matches!(fields.as_slice(), [(name, TypeToken::MixedEnum { .. })] if name == "0") => {
            wrapper(node, facts, token).map(Some)
        }
        _ => Ok(None),
    }
}

fn named<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
    token: TypeToken,
) -> Result<Constructor<'a>, String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;

    let syn::Expr::Struct(syntax) = syntax else {
        return Err("Named error variant lost original construction syntax".into());
    };

    let (owner, variant) = source::variant(&syntax)?;

    let resolution = facts.nominal_resolution()?;

    let tree = resolution.resolve(&syn::TypePath {
        qself: None,
        path: owner,
    })?;

    if TypeToken::from_tree(&tree) != token {
        return Err("Named error owner differs from complete captured type".into());
    }

    let TypeTree::MixedEnum {
        nominal, variants, ..
    } = tree
    else {
        return Err("Named error requires a complete mixed enum".into());
    };

    if !resolution.checked_no_drop(&nominal) {
        return Err("Named error lacks complete independent no-drop proof".into());
    }

    let selected = variants
        .iter()
        .find(|entry| entry.name == variant)
        .ok_or("Unknown named error variant")?;

    let VariantShape::Named { fields } = &selected.shape else {
        return Err("Named error selected a different variant shape".into());
    };

    let operands = field_operands(node, facts, &syntax, fields)?;

    Ok(Constructor::Named {
        token,
        nominal,
        variant,
        fields: operands,
    })
}

fn wrapper<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
    token: TypeToken,
) -> Result<Constructor<'a>, String> {
    let resolution = facts.nominal_resolution()?;

    let TypeToken::Struct { nominal, .. } = &token else {
        unreachable!()
    };

    if !resolution.checked_no_drop(nominal) {
        return Err("Fresh error wrapper lacks complete no-drop proof".into());
    }

    let slots = crate::data_structs::slots(node, facts)?;

    let [payload] = node.children.as_slice() else {
        return Err("Fresh error wrapper requires exactly one whole payload".into());
    };

    if slots != [Some(0)] {
        return Err("Fresh error wrapper changes its unary field".into());
    }

    Ok(Constructor::Wrapper { token, payload })
}

fn field_operands<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
    syntax: &syn::ExprStruct,
    fields: &[(String, crate::concrete_types::Field)],
) -> Result<Vec<FieldOperand<'a>>, String> {
    field_shape(fields, facts)?;
    let declared: Vec<_> = fields.iter().map(|(name, _)| name.as_str()).collect();

    let slots = source::slots(syntax, &declared)?;

    if node.children.len() != slots.len()
        || node.dispatch.is_some()
        || node.res.is_some()
        || node.is_unsafe_block
    {
        return Err("Named error field evaluations disagree with original syntax".into());
    }

    let mut operands = Vec::new();
    for ((syntax_field, operand), position) in syntax.fields.iter().zip(&node.children).zip(slots) {
        let (name, field) = &fields[position];

        let expected = TypeToken::from_tree(&field.ty);
        let declaration =
            syn::parse_str::<syn::Type>(&field.declaration).map_err(|e| e.to_string())?;
        source::payload_type(&declaration, &expected)?;
        if expected == TypeToken::Scalar("u32".into())
            && !crate::extraction_profile::facts_support(facts, SCALAR_CAPABILITY)
            || crate::data_aggregates::parsed_type(operand)? != expected
        {
            return Err("Named error payload is outside its exact supported field type".into());
        }

        crate::source_evidence::verify_child_source(&syntax_field.expr, operand, node, facts)?;

        let formal = formals::formal_operand(operand, facts)?;
        operands.push(FieldOperand {
            position,
            name: name.clone(),
            operand,
            formal,
        });
    }

    Ok(operands)
}

fn field_shape(
    fields: &[(String, crate::concrete_types::Field)],
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let types: Vec<_> = fields
        .iter()
        .map(|(_, field)| TypeToken::from_tree(&field.ty))
        .collect();
    let static_string = TypeToken::Shared(Box::new(TypeToken::Str));
    if types == [static_string]
        || crate::extraction_profile::facts_support(facts, SCALAR_CAPABILITY)
            && types
                == [
                    TypeToken::Scalar("u32".into()),
                    TypeToken::Scalar("u32".into()),
                ]
    {
        Ok(())
    } else {
        Err("Named error requires one static-string field or two builtin-u32 fields".into())
    }
}
