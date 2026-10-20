//! Rebuild body-only nominal trees from existing tokens and declaration ranges.

use super::*;
use crate::{HirFnFacts, HirNode, data_types::TypeToken};
use std::borrow::Cow;

/// A temporary declaration inventory; added observations are never callable parameters.
/// Existing interface capture and serialized records remain unchanged.
pub fn context<'a>(
    facts: &HirFnFacts,
    interface: &'a InterfaceObservations,
    origins: &Origins,
) -> Result<Cow<'a, InterfaceObservations>, String> {
    if !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .supports("rust.body-nominal-types")
        || origins.declarations.is_empty()
    {
        return Ok(Cow::Borrowed(interface));
    }
    let expected = declarations(interface)?;
    if origins
        .declarations
        .iter()
        .all(|origin| expected.contains_key(&origin.nominal.definition_path))
    {
        return Ok(Cow::Borrowed(interface));
    }

    let mut tokens = BTreeMap::new();
    nodes(&facts.body, &mut tokens)?;
    let mut context = interface.clone();
    for (name, token) in tokens {
        if !expected.contains_key(&name) {
            context.parameters.push(Observation::Captured {
                ty: tree(&token, origins)?,
            });
        }
    }
    context.validate()?;
    Ok(Cow::Owned(context))
}

fn nodes(node: &HirNode, tokens: &mut BTreeMap<String, TypeToken>) -> Result<(), String> {
    if let Some(text) = node.ty.as_ref().and_then(|ty| ty.normalized.as_deref())
        && text.contains("\"kind\":\"struct\"")
    {
        collect_tokens(&crate::data_types::parse(text)?, tokens)?;
    }
    for child in &node.children {
        nodes(child, tokens)?;
    }
    Ok(())
}

fn collect_tokens(
    token: &TypeToken,
    tokens: &mut BTreeMap<String, TypeToken>,
) -> Result<(), String> {
    match token {
        TypeToken::Struct { .. } => nominal_tokens(token, tokens)?,
        TypeToken::Tuple(elements) => {
            for element in elements {
                collect_tokens(element, tokens)?;
            }
        }
        TypeToken::Shared(element)
        | TypeToken::Slice(element)
        | TypeToken::Array { element, .. } => collect_tokens(element, tokens)?,
        _ => {}
    }
    Ok(())
}

fn nominal_tokens(
    token: &TypeToken,
    tokens: &mut BTreeMap<String, TypeToken>,
) -> Result<(), String> {
    let TypeToken::Struct { nominal, fields } = token else {
        return Err("Expected a nominal body type".into());
    };
    if let Some(previous) = tokens.get(&nominal.definition_path) {
        return if previous == token {
            Ok(())
        } else {
            Err("Conflicting body nominal type observations".into())
        };
    }

    tokens.insert(nominal.definition_path.clone(), token.clone());
    for (_, field) in fields {
        collect_tokens(field, tokens)?;
    }
    Ok(())
}

fn tree(token: &TypeToken, origins: &Origins) -> Result<TypeTree, String> {
    Ok(match token {
        TypeToken::Scalar(value) => TypeTree::Scalar(value.clone()),
        TypeToken::Str => TypeTree::Str,
        TypeToken::SliceArrayResult { length, borrowed } => TypeTree::SliceArrayResult {
            length: *length,
            borrowed: *borrowed,
        },
        TypeToken::Shared(element) => TypeTree::Shared(Box::new(tree(element, origins)?)),
        TypeToken::Slice(element) => TypeTree::Slice(Box::new(tree(element, origins)?)),
        TypeToken::Array { element, length } => TypeTree::Array {
            element: Box::new(tree(element, origins)?),
            length: *length,
        },
        TypeToken::Tuple(elements) => TypeTree::Tuple(
            elements
                .iter()
                .map(|element| tree(element, origins))
                .collect::<Result<_, _>>()?,
        ),
        TypeToken::Struct { nominal, fields } => structure(nominal, fields, origins)?,
    })
}

fn structure(
    nominal: &Nominal,
    fields: &[(String, TypeToken)],
    origins: &Origins,
) -> Result<TypeTree, String> {
    let origin = origins
        .declarations
        .iter()
        .find(|origin| origin.nominal == *nominal)
        .ok_or("Body nominal type lacks its original declaration")?;
    let source = origin
        .file_bytes
        .get(origin.declaration_span.start as usize..origin.declaration_span.end as usize)
        .ok_or("Body nominal declaration range is invalid")?;
    let declarations = crate::concrete_types::field_declarations(source)?;
    if declarations.len() != fields.len() {
        return Err("Body nominal field count disagrees with declaration".into());
    }

    let fields = declarations
        .into_iter()
        .zip(fields)
        .enumerate()
        .map(|(position, ((name, declaration), (observed, ty)))| {
            if name != *observed {
                return Err("Body nominal field name/order disagrees with declaration".into());
            }
            Ok(crate::concrete_types::Field {
                position,
                name,
                declaration: declaration.into(),
                ty: tree(ty, origins)?,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(TypeTree::Struct {
        nominal: nominal.clone(),
        source: source.into(),
        fields,
    })
}
