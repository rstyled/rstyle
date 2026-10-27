//! Literal character intervals retain source endpoints and the Unicode scalar gap.

use crate::{HirNode, scalar_patterns::IntegerRange, unit_store::EvidenceFacts};
use syn::{parse::Parser, spanned::Spanned};

pub const CAPABILITY: &str = "rust.character-literal-ranges";

pub fn from_source(source: &str) -> Result<IntegerRange, String> {
    let mut pattern = syn::Pat::parse_single
        .parse_str(source)
        .map_err(|e| e.to_string())?;
    while let syn::Pat::Paren(parenthesized) = pattern {
        pattern = *parenthesized.pat;
    }
    let syn::Pat::Range(range) = pattern else {
        return Err("Expected an original character range".into());
    };

    let start = range
        .start
        .as_deref()
        .map(|bound| endpoint(source, bound))
        .transpose()?;
    let end = range
        .end
        .as_deref()
        .map(|bound| endpoint(source, bound))
        .transpose()?;
    let inclusive = matches!(range.limits, syn::RangeLimits::Closed(_));
    if start.is_none() && end.is_none() || inclusive && end.is_none() {
        return Err("Character range requires definite endpoints".into());
    }
    let low = start
        .as_deref()
        .and_then(crate::character_scalars::value)
        .unwrap_or('\0');
    let high = end
        .as_deref()
        .and_then(crate::character_scalars::value)
        .unwrap_or(char::MAX);
    if low > high || !inclusive && end.is_some() && low == high {
        return Err("Character range is empty or reversed".into());
    }
    Ok(IntegerRange {
        start,
        end,
        inclusive,
    })
}

fn endpoint(source: &str, expression: &syn::Expr) -> Result<String, String> {
    if !matches!(expression, syn::Expr::Lit(value) if matches!(value.lit, syn::Lit::Char(_))) {
        return Err("Character range endpoints must be literal Unicode scalars".into());
    }

    let span = expression.span();
    let start = crate::source_evidence::byte_offset(source, span.start());
    let end = crate::source_evidence::byte_offset(source, span.end());
    let spelling = source
        .get(start..end)
        .ok_or("Character endpoint span is unavailable")?;
    crate::character_scalars::value(spelling).ok_or("Invalid Unicode scalar endpoint")?;
    Ok(spelling.into())
}

pub(crate) fn verify_node(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if node.kind != "pat_range" || crate::frontend::normalized(node) != Some("char") {
        return Ok(());
    }
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || node.expansion.is_some()
        || node.is_unsafe_block
        || !node.children.is_empty()
    {
        return Err(
            "Character ranges require registered original literal endpoint evidence".into(),
        );
    }

    let source =
        crate::byte_literals::source(node, facts).ok_or("Missing character range source")?;
    if node.lit.as_deref() != Some(from_source(source)?.text().as_str()) {
        return Err("Character range loses original endpoints, order or inclusivity".into());
    }
    Ok(())
}

pub(crate) fn uses(node: &HirNode) -> bool {
    (node.kind == "pat_range" && crate::frontend::normalized(node) == Some("char"))
        || node.children.iter().any(uses)
}

pub(crate) fn selector(
    pattern: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(&'static str, Option<String>), String> {
    let mut leaves = Vec::new();
    collect(pattern, facts, &mut leaves)?;

    if pattern.kind != "pat_or" {
        return Ok(("match.range", pattern.lit.clone()));
    }
    let values: Vec<_> = leaves.iter().map(|leaf| (&leaf.kind, &leaf.lit)).collect();
    Ok((
        "match.or-patterns",
        Some(serde_json::to_string(&values).map_err(|e| e.to_string())?),
    ))
}

fn collect<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
    leaves: &mut Vec<&'a HirNode>,
) -> Result<(), String> {
    if crate::frontend::normalized(node) != Some("char") {
        return Err("Character selector loses its scalar domain".into());
    }
    match node.kind.as_str() {
        "pat_or" => {
            for child in &node.children {
                collect(child, facts, leaves)?;
            }
        }
        "pat_range" => {
            verify_node(node, facts)?;
            leaves.push(node);
        }
        "pat_literal" | "pat_wild" => {
            crate::character_scalars::verify_node(node, facts)?;
            leaves.push(node);
        }
        _ => return Err("Character range selector has unsupported neighbors".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::from_source;

    #[test]
    fn retains_escaped_bounds_and_inclusivity() {
        let range = from_source(r"'\u{D7FF}'..='\u{E000}'").unwrap();
        assert_eq!(range.start.as_deref(), Some(r"'\u{D7FF}'"));
        assert_eq!(range.end.as_deref(), Some(r"'\u{E000}'"));
        assert!(range.inclusive);
        assert!(!from_source("'a'..'z'").unwrap().inclusive);
        assert!(from_source("..='a'").unwrap().start.is_none());
        assert!(from_source("'a'..").unwrap().end.is_none());
    }

    #[test]
    fn rejects_unknown_mixed_empty_and_reversed_bounds() {
        for source in [
            "'z'..='a'",
            "'a'..'a'",
            "..'\\0'",
            "0..='z'",
            "'a'..=END",
            "..",
            r"'\u{D800}'..='z'",
        ] {
            assert!(from_source(source).is_err(), "{source}");
        }
    }
}
