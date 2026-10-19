//! Source-ordered tuple and array literal values without stored references.

use crate::{ByteSpan, HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

pub(crate) fn constructor(node: &HirNode) -> bool {
    matches!(node.kind.as_str(), "expr_array" | "expr_repeat")
        || node.kind == "expr_tup" && !node.children.is_empty()
}

pub(crate) fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let ty = crate::data_types::parse(token(node)?)?;
    if !reference_free(&ty) {
        return Err("Aggregate construction requires reference-free component values".into());
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

fn token(node: &HirNode) -> Result<&str, String> {
    node.ty
        .as_ref()
        .and_then(|ty| ty.normalized.as_deref())
        .ok_or_else(|| "Missing concrete aggregate value type".into())
}

fn reference_free(ty: &TypeToken) -> bool {
    match ty {
        TypeToken::Scalar(_) => true,
        TypeToken::Tuple(elements) => elements.iter().all(reference_free),
        TypeToken::Array { element, .. } => reference_free(element),
        TypeToken::Struct { fields, .. } => fields.iter().all(|(_, ty)| reference_free(ty)),
        _ => false,
    }
}

fn ungroup(mut expression: &syn::Expr) -> &syn::Expr {
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

    match crate::data_types::parse(text) {
        Ok(TypeToken::Tuple(elements)) if node.original_kind.known_value() == Some("expr_tup") => {
            usize::try_from(position)
                .ok()
                .and_then(|index| elements.get(index))
                == Some(&TypeToken::Scalar("unit".into()))
        }
        Ok(TypeToken::Array { element, length }) => {
            *element == TypeToken::Scalar("unit".into())
                && match node.original_kind.known_value() {
                    Some("expr_array") => position < length,
                    Some("expr_repeat") => {
                        position == 0
                            && crate::extraction_profile::policy_supports(
                                &pdg.profile,
                                "rust.repeat-array-values",
                            )
                    }
                    _ => false,
                }
        }
        _ => false,
    }
}
