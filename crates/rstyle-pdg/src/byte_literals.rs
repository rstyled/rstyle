//! Byte syntax denotes a concrete u8; source spelling and escape values stay checked.

use crate::{HirNode, unit_store::EvidenceFacts};
use syn::parse::Parser;

pub const CAPABILITY: &str = "rust.byte-scalar-literals";

pub fn value(source: &str) -> Option<u8> {
    if !source.trim_start().starts_with("b'") {
        return None;
    }
    let literal: syn::LitByte = syn::parse_str(source).ok()?;
    literal.suffix().is_empty().then(|| literal.value())
}

pub(crate) fn source<'a>(node: &HirNode, facts: &'a dyn EvidenceFacts) -> Option<&'a str> {
    facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
}

pub(crate) fn uses(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    let Some(source) = source(node, facts) else {
        return false;
    };
    match node.kind.as_str() {
        "expr_lit" | "pat_literal" => value(source).is_some(),
        "pat_or" => node.children.iter().any(|child| uses(child, facts)),
        "pat_range" if source.contains("b'") => {
            let Ok(syn::Pat::Range(range)) = syn::Pat::parse_single.parse_str(source) else {
                return false;
            };
            range.start.iter().chain(&range.end).any(|bound| {
                matches!(&**bound, syn::Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Byte(_)))
            })
        }
        _ => false,
    }
}

pub(crate) fn verify_node(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !uses(node, facts) {
        return Ok(());
    }
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some("u8")
        || node.expansion.is_some()
        || node.is_unsafe_block
    {
        return Err(
            "Byte syntax requires its registered policy and original concrete u8 evidence".into(),
        );
    }
    if matches!(node.kind.as_str(), "expr_lit" | "pat_literal")
        && (!node.children.is_empty() || node.lit.as_deref() != source(node, facts))
    {
        return Err("Byte literal loses its complete original spelling/value".into());
    }
    Ok(())
}
