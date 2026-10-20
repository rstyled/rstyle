//! Built-in shared dereference reads reuse formal physical locations.

use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    node.kind == "expr_unary"
        && node
            .dispatch
            .as_ref()
            .is_some_and(|dispatch| dispatch.op == "*")
        && crate::extraction_profile::ExtractionProfile::from_facts(facts)
            .is_ok_and(|profile| profile.supports("rust.shared-dereference-reads"))
}

pub(crate) fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !candidate(node, facts)
        || !node
            .dispatch
            .as_ref()
            .is_some_and(|dispatch| dispatch.kind == "builtin" && dispatch.operand_tys.len() == 1)
    {
        return Err("Dereference requires registered built-in shared read semantics".into());
    }
    let [base] = node.children.as_slice() else {
        return Err("Dereference lacks one evaluated pointer".into());
    };
    let TypeToken::Shared(referent) =
        crate::data_types::parse(crate::data_aggregates::token(base)?)?
    else {
        return Err("Dereference requires a shared reference value".into());
    };
    if matches!(referent.as_ref(), TypeToken::Slice(_) | TypeToken::Str)
        || crate::data_types::parse(crate::data_aggregates::token(node)?)? != *referent
    {
        return Err("Dereference result lacks a supported sized referent type".into());
    }

    verify_source(node, base, facts)
}

fn verify_source(node: &HirNode, base: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let bytes = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original dereference source")?;
    let expression: syn::Expr = syn::parse_str(bytes).map_err(|error| error.to_string())?;
    let syn::Expr::Unary(expression) = crate::data_aggregates::ungroup(&expression) else {
        return Err("Dereference lacks original unary syntax".into());
    };
    if !matches!(expression.op, syn::UnOp::Deref(_)) {
        return Err("Dereference disagrees with original operator".into());
    }

    let span = crate::data_aggregates::ungroup(&expression.expr).span();
    let syntax = crate::ByteSpan {
        start: node.span.start + crate::source_evidence::byte_offset(bytes, span.start()) as u32,
        end: node.span.start + crate::source_evidence::byte_offset(bytes, span.end()) as u32,
    };
    if !crate::source_evidence::corresponding_span(&facts.source.bytes, syntax, base.span) {
        return Err("Dereference loses its evaluated pointer correspondence".into());
    }
    Ok(())
}
