//! Select a component of an evaluated aggregate, without a memory location.

use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use syn::{ext::IdentExt, spanned::Spanned};

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    if crate::range_values::value_field(node, facts).is_ok_and(|position| position.is_some()) {
        return true;
    }
    if node.kind != "expr_field"
        || !crate::extraction_profile::ExtractionProfile::from_facts(facts)
            .is_ok_and(|profile| profile.supports("rust.aggregate-value-fields"))
    {
        return false;
    }
    let [base] = node.children.as_slice() else {
        return false;
    };

    if crate::data_dereferences::candidate(base, facts) {
        return false;
    }

    let evaluated = match base.kind.as_str() {
        "expr_path" => base
            .res
            .as_ref()
            .and_then(|resolution| resolution.target.as_ref())
            .is_some_and(|target| {
                facts.bindings.iter().any(|binding| {
                    binding.hir == *target
                        && (!binding.is_param
                            || (binding.is_mut && reassigned(&facts.body, target)))
                })
            }),
        "expr_field" => candidate(base, facts),
        _ => true,
    };

    evaluated
        && crate::data_aggregates::token(base)
            .ok()
            .and_then(|token| crate::data_types::parse(token).ok())
            .is_some_and(|ty| {
                matches!(ty, TypeToken::Tuple(_) | TypeToken::Struct { .. })
                    && (crate::data_aggregates::reference_free(&ty)
                        || crate::extraction_profile::ExtractionProfile::from_facts(facts)
                            .is_ok_and(|profile| profile.supports("rust.reference-value-fields")))
            })
}

fn reassigned(node: &HirNode, target: &crate::HirRef) -> bool {
    (node.kind == "expr_assign"
        && node.children.first().is_some_and(|lhs| {
            lhs.kind == "expr_path"
                && lhs
                    .res
                    .as_ref()
                    .and_then(|resolution| resolution.target.as_ref())
                    == Some(target)
        }))
        || node.children.iter().any(|child| reassigned(child, target))
}

pub(crate) fn index(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<usize, String> {
    if let Some(position) = crate::range_values::value_field(node, facts)? {
        return Ok(position);
    }
    if !candidate(node, facts) {
        return Err("Field lacks a supported aggregate value base".into());
    }
    let base = &node.children[0];
    let source = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original value field source")?;
    let expression: syn::Expr = syn::parse_str(source).map_err(|error| error.to_string())?;
    let syn::Expr::Field(field) = crate::data_aggregates::ungroup(&expression) else {
        return Err("Expected original field selection".into());
    };
    let resolution = node
        .res
        .as_ref()
        .filter(|resolution| resolution.kind == "resolved_field")
        .ok_or("Missing resolved value field identity")?;
    let resolution: crate::field_locations::FieldResolution =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    let token = crate::data_types::parse(crate::data_aggregates::token(base)?)?;
    let (position, expected) = component(&token, &field.member, &resolution)?;
    if expected != &crate::data_types::parse(crate::data_aggregates::token(node)?)? {
        return Err("Value field output disagrees with its aggregate component".into());
    }

    let span = crate::data_aggregates::ungroup(&field.base).span();
    let syntax = crate::ByteSpan {
        start: node.span.start + crate::source_evidence::byte_offset(source, span.start()) as u32,
        end: node.span.start + crate::source_evidence::byte_offset(source, span.end()) as u32,
    };
    if !crate::source_evidence::corresponding_span(&facts.source.bytes, syntax, base.span) {
        return Err("Value field loses its original evaluated base".into());
    }
    Ok(position)
}

fn component<'a>(
    ty: &'a TypeToken,
    member: &syn::Member,
    resolution: &crate::field_locations::FieldResolution,
) -> Result<(usize, &'a TypeToken), String> {
    let found = match (ty, member) {
        (TypeToken::Tuple(elements), syn::Member::Unnamed(index))
            if resolution.owner_path.is_none() =>
        {
            elements
                .get(index.index as usize)
                .map(|ty| (index.index as usize, ty))
        }
        (TypeToken::Struct { nominal, fields }, member)
            if resolution.owner_path.as_ref() == Some(&nominal.definition_path) =>
        {
            let name = match member {
                syn::Member::Named(name) => name.unraw().to_string(),
                syn::Member::Unnamed(index) => index.index.to_string(),
            };
            fields
                .iter()
                .enumerate()
                .find(|(_, (field, _))| *field == name)
                .map(|(index, (_, ty))| (index, ty))
        }
        _ => None,
    }
    .ok_or("Value field source member disagrees with its exact aggregate owner")?;

    if found.0 != resolution.position {
        return Err("Value field source/resolved positions disagree".into());
    }
    Ok(found)
}
