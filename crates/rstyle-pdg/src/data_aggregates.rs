//! Source-ordered aggregate literal values with exact typed component slots.

use crate::{ByteSpan, HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

pub(crate) fn constructor(node: &HirNode) -> bool {
    matches!(
        node.kind.as_str(),
        "expr_array" | "expr_repeat" | "expr_struct"
    ) || node.kind == "expr_tup" && !node.children.is_empty()
}

pub(crate) fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let ty = crate::data_types::parse(token(node)?)?;
    let references = !reference_free(&ty);
    if references && !reference_literal(node, facts) {
        return Err(
            "Reference-bearing construction requires its registered literal capability".into(),
        );
    }

    if node.kind == "expr_struct" {
        return verify_struct(node, facts, references);
    }

    let length = match &ty {
        TypeToken::Tuple(elements) if node.kind == "expr_tup" => Some(elements.len()),
        TypeToken::Array { .. } if node.kind == "expr_repeat" => Some(1),
        TypeToken::Array { length, .. } if node.kind == "expr_array" => {
            usize::try_from(*length).ok()
        }
        _ => return Err("Aggregate constructor kind disagrees with its resolved type".into()),
    };
    if length != Some(node.children.len()) {
        return Err("Aggregate constructor arity disagrees with its resolved type".into());
    }

    for (index, child) in node.children.iter().enumerate() {
        let expected = match &ty {
            TypeToken::Tuple(elements) => &elements[index],
            TypeToken::Array { element, .. } => &**element,
            _ => unreachable!("checked constructor type"),
        };
        if expected != &crate::data_types::parse(token(child)?)?
            || child.ty.as_ref().is_some_and(|ty| ty.observed == "!")
        {
            return Err("Aggregate component type disagrees with its resolved position".into());
        }
    }
    verify_source(node, facts)
}

fn verify_struct(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    references: bool,
) -> Result<(), String> {
    let slots = crate::data_structs::slots(node, facts)?;
    if references
        && slots.contains(&None)
        && !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
            .supports("rust.reference-struct-updates")
    {
        return Err(
            "Reference-bearing struct updates need component-specific origin substitution".into(),
        );
    }
    Ok(())
}

// Verified struct slots permit a same-typed operand only for the update base.
pub(crate) fn update(node: &HirNode) -> bool {
    node.kind == "expr_struct"
        && node.children.last().is_some_and(|child| {
            child
                .ty
                .as_ref()
                .and_then(|ty| ty.normalized.as_deref())
                .is_some()
                && child.ty.as_ref().and_then(|ty| ty.normalized.as_deref())
                    == node.ty.as_ref().and_then(|ty| ty.normalized.as_deref())
        })
}

pub(crate) fn reference_literal(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    matches!(node.kind.as_str(), "expr_tup" | "expr_struct")
        && crate::extraction_profile::ExtractionProfile::from_facts(facts)
            .is_ok_and(|profile| profile.supports("rust.reference-bearing-values"))
}

pub(crate) fn token(node: &HirNode) -> Result<&str, String> {
    node.ty
        .as_ref()
        .and_then(|ty| ty.normalized.as_deref())
        .ok_or_else(|| "Missing concrete aggregate value type".into())
}

pub(crate) fn reference_free(ty: &TypeToken) -> bool {
    match ty {
        TypeToken::Scalar(_) | TypeToken::IntConversionResult(_) => true,
        TypeToken::SliceArrayResult {
            borrowed: false, ..
        } => true,
        TypeToken::Tuple(elements) => elements.iter().all(reference_free),
        TypeToken::Array { element, .. } => reference_free(element),
        TypeToken::Option(payload) => matches!(payload.as_ref(), TypeToken::Scalar(_)),
        TypeToken::Struct { fields, .. } => fields.iter().all(|(_, ty)| reference_free(ty)),
        TypeToken::Enum { variants, .. } => variants
            .iter()
            .all(|(_, ty)| matches!(ty, TypeToken::Scalar(_))),
        _ => false,
    }
}

pub(crate) fn ungroup(mut expression: &syn::Expr) -> &syn::Expr {
    while let syn::Expr::Paren(group) = expression {
        expression = &group.expr;
    }
    expression
}

fn verify_source(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let source = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original aggregate source bytes")?;
    let syntax: syn::Expr = syn::parse_str(source).map_err(|error| error.to_string())?;

    if let syn::Expr::Repeat(repeat) = ungroup(&syntax) {
        return verify_repeat(node, repeat, source, facts);
    }
    let elements = match ungroup(&syntax) {
        syn::Expr::Tuple(tuple) if node.kind == "expr_tup" => &tuple.elems,
        syn::Expr::Array(array) if node.kind == "expr_array" => &array.elems,
        _ => return Err("Aggregate source syntax disagrees with compiler constructor".into()),
    };
    if elements.len() != node.children.len() {
        return Err("Aggregate source loses or invents component evaluations".into());
    }

    for (syntax, child) in elements.iter().zip(&node.children) {
        let span = ungroup(syntax).span();
        let syntax = ByteSpan {
            start: node.span.start
                + crate::source_evidence::byte_offset(source, span.start()) as u32,
            end: node.span.start + crate::source_evidence::byte_offset(source, span.end()) as u32,
        };
        if !crate::source_evidence::corresponding_span(&facts.source.bytes, syntax, child.span) {
            return Err("Aggregate component source order/correspondence disagrees".into());
        }
    }
    Ok(())
}

fn verify_repeat(
    node: &HirNode,
    syntax: &syn::ExprRepeat,
    source: &str,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    if node.kind != "expr_repeat"
        || !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
            .supports("rust.repeat-array-values")
    {
        return Err("Repeat array requires its registered capability".into());
    }
    let syn::Expr::Lit(count) = ungroup(&syntax.len) else {
        return Err("Repeat count requires a literal integer".into());
    };
    let syn::Lit::Int(count) = &count.lit else {
        return Err("Repeat count requires a literal integer".into());
    };
    let TypeToken::Array { length, .. } = crate::data_types::parse(token(node)?)? else {
        return Err("Repeat result is not an array".into());
    };
    if !matches!(count.suffix(), "" | "usize") || count.base10_parse::<u64>().ok() != Some(length) {
        return Err("Repeat count disagrees with resolved array length".into());
    }

    let span = ungroup(&syntax.expr).span();
    let syntax = ByteSpan {
        start: node.span.start + crate::source_evidence::byte_offset(source, span.start()) as u32,
        end: node.span.start + crate::source_evidence::byte_offset(source, span.end()) as u32,
    };
    if !crate::source_evidence::corresponding_span(
        &facts.source.bytes,
        syntax,
        node.children[0].span,
    ) {
        return Err("Repeat operand loses its original single evaluation".into());
    }
    Ok(())
}

/// Unit values exist only as explicitly typed constructor slots, never scalar operands.
pub(crate) fn unit_slot(pdg: &crate::PdgDocument, edge: &crate::PdgEdge) -> bool {
    if edge.subkind != "value_operand"
        || !crate::extraction_profile::policy_supports(
            &pdg.profile,
            "rust.aggregate-literal-values",
        )
    {
        return false;
    }
    let Some(node) = pdg.nodes.iter().find(|node| node.id == edge.target) else {
        return false;
    };
    if node.category != crate::NodeCategory::Computation {
        return false;
    }
    let Some(position) = edge.operand_position else {
        return false;
    };
    let Some(text) = node
        .annotations
        .get("compat.rust.hir-r0:type")
        .and_then(|fact| fact.get("value"))
        .and_then(serde_json::Value::as_str)
    else {
        return false;
    };

    crate::data_types::parse(text).is_ok_and(|ty| unit_component(ty, node, position, &pdg.profile))
}

fn unit_component(
    ty: TypeToken,
    node: &crate::PdgNode,
    position: u64,
    profile: &crate::ProfileId,
) -> bool {
    match ty {
        TypeToken::Tuple(elements) if node.original_kind.known_value() == Some("expr_tup") => {
            usize::try_from(position)
                .ok()
                .and_then(|index| elements.get(index))
                == Some(&TypeToken::Scalar("unit".into()))
        }
        TypeToken::Array { element, length } => {
            *element == TypeToken::Scalar("unit".into())
                && match node.original_kind.known_value() {
                    Some("expr_array") => position < length,
                    Some("expr_repeat") => {
                        position == 0
                            && crate::extraction_profile::policy_supports(
                                profile,
                                "rust.repeat-array-values",
                            )
                    }
                    _ => false,
                }
        }
        TypeToken::Struct { fields, .. }
            if node.original_kind.known_value() == Some("expr_struct")
                && crate::extraction_profile::policy_supports(
                    profile,
                    "rust.named-struct-values",
                ) =>
        {
            struct_unit_slot(node, position, &fields)
        }
        _ => false,
    }
}

fn struct_unit_slot(node: &crate::PdgNode, position: u64, fields: &[(String, TypeToken)]) -> bool {
    let Some(text) = node
        .attributes
        .literal
        .as_ref()
        .and_then(|literal| literal.known_value())
    else {
        return false;
    };
    let Ok(slots) = serde_json::from_str::<Vec<Option<usize>>>(text) else {
        return false;
    };
    usize::try_from(position)
        .ok()
        .and_then(|index| slots.get(index))
        .and_then(|index| index.and_then(|index| fields.get(index)))
        .is_some_and(|(_, ty)| ty == &TypeToken::Scalar("unit".into()))
}

/// Parse the canonical concrete type attached to a source value.
pub(crate) fn parsed_type(node: &HirNode) -> Result<TypeToken, String> {
    crate::data_types::parse(token(node)?)
}
