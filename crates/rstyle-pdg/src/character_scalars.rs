//! Unicode scalar literals and builtin comparisons retain their distinct domain.

use crate::frontend::normalized;
use crate::{HirNode, unit_store::EvidenceFacts};

pub const CAPABILITY: &str = "rust.character-scalars";

pub fn value(source: &str) -> Option<char> {
    let literal: syn::LitChar = syn::parse_str(source).ok()?;
    literal.suffix().is_empty().then(|| literal.value())
}

pub(crate) fn verify_node(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let source = crate::byte_literals::source(node, facts);
    let literal = matches!(node.kind.as_str(), "expr_lit" | "pat_literal");
    let character = normalized(node) == Some("char");
    if !character && !(literal && source.and_then(value).is_some()) {
        return Ok(());
    }

    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || !character
        || node.expansion.is_some()
        || node.is_unsafe_block
    {
        return Err("Character values require registered original Unicode scalar evidence".into());
    }
    if literal {
        verify_literal(node, source.ok_or("Missing original character literal")?)?;
    }
    if node.kind == "pat_range" {
        return Err("Character ranges require a separate domain contract".into());
    }
    Ok(())
}

fn verify_literal(node: &HirNode, source: &str) -> Result<(), String> {
    if normalized(node) != Some("char")
        || !node.children.is_empty()
        || node.lit.as_deref() != Some(source)
        || value(source).is_none()
    {
        return Err("Character literal loses its original spelling, type or scalar value".into());
    }
    Ok(())
}

pub(crate) fn comparison(node: &HirNode) -> Result<(), String> {
    let [left, right] = node.children.as_slice() else {
        return Err("Character comparison requires two saved operands".into());
    };
    if normalized(left) != Some("char")
        || normalized(right) != Some("char")
        || normalized(node) != Some("bool")
        || node.kind != "expr_binary"
        || !node.dispatch.as_ref().is_some_and(|dispatch| {
            dispatch.kind == "builtin"
                && ["==", "!=", "<", "<=", ">", ">="].contains(&dispatch.op.as_str())
                && dispatch.operand_tys == ["char", "char"]
                && dispatch.result_ty == "bool"
        })
    {
        return Err(
            "Character comparison requires exact builtin char operands and bool result".into(),
        );
    }
    Ok(())
}

pub(crate) fn verify_document_policy(
    policy: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        policy,
        CAPABILITY,
        || character_function(facts),
        "Character scalars require their registered document policy",
    )
}

pub(crate) fn verify_function(facts: &dyn EvidenceFacts) -> Result<(), String> {
    if character_function(facts) && !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Character scalars require their registered extraction policy".into());
    }
    Ok(())
}

fn character_function(facts: &dyn EvidenceFacts) -> bool {
    facts
        .signature
        .params
        .iter()
        .any(|parameter| parameter.ty.normalized.as_deref() == Some("char"))
        || facts.signature.return_ty.normalized.as_deref() == Some("char")
        || facts
            .bindings
            .iter()
            .any(|binding| binding.ty.normalized.as_deref() == Some("char"))
        || character_node(&facts.body)
}

fn character_node(node: &HirNode) -> bool {
    normalized(node) == Some("char") || node.children.iter().any(character_node)
}

#[cfg(test)]
mod tests {
    use super::value;

    #[test]
    fn decodes_unicode_scalars_without_integer_or_byte_coercion() {
        for (source, expected) in [
            ("'a'", 'a'),
            (r"'\\'", '\\'),
            (r"'\n'", '\n'),
            (r"'\x41'", 'A'),
            (r"'\u{D7FF}'", '\u{D7FF}'),
            (r"'\u{E000}'", '\u{E000}'),
            (r"'\u{10FFFF}'", '\u{10FFFF}'),
            ("'😀'", '😀'),
        ] {
            assert_eq!(value(source), Some(expected), "{source}");
        }
        for source in [
            "97",
            "b'a'",
            "\"a\"",
            "'ab'",
            "''",
            r"'\u{D800}'",
            r"'\u{110000}'",
            r"'\xFF'",
            "'a'u32",
            "'a' + 'b'",
        ] {
            assert_eq!(value(source), None, "{source}");
        }
    }
}
