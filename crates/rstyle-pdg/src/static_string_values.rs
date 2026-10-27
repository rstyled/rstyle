//! Exact source-owned static string identity; escaping routes need caller proof.
use crate::{ByteSpan, HirNode, HirRef, unit_store::EvidenceFacts};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

pub(crate) const CAPABILITY: &str = "rust.named-error-validator-flow";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Descriptor {
    pub source_digest: String,
    pub hir: HirRef,
    pub span: ByteSpan,
    pub spelling: String,
    pub value: String,
}

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    node.kind == "expr_lit"
        && crate::extraction_profile::facts_support(facts, CAPABILITY)
        && crate::data_aggregates::parsed_type(node).is_ok_and(|ty| {
            ty == crate::data_types::TypeToken::Shared(Box::new(crate::data_types::TypeToken::Str))
        })
}

pub(crate) fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<Descriptor, String> {
    let spelling = crate::source_evidence::byte_range(&facts.source.bytes, node.span)?;
    let literal: syn::LitStr = syn::parse_str(spelling).map_err(|e| e.to_string())?;
    if !literal.suffix().is_empty()
        || !candidate(node, facts)
        || node.expansion.is_some()
        || node.is_unsafe_block
        || node.res.is_some()
        || node.dispatch.is_some()
        || !node.children.is_empty()
        || node.binding.is_some()
        || node.binding_mut.is_some()
        || node.param_index.is_some()
        || node.hir.krate != 0
        || node.hir.owner != facts.body.hir.owner
        || node.snippet != spelling
        || node.lit.as_deref() != Some(spelling)
        || spelling.len() > 4096
    {
        return Err("Static string value loses its exact original owned literal identity".into());
    }

    Ok(Descriptor {
        source_digest: facts.source.digest.clone(),
        hir: node.hir.clone(),
        span: node.span,
        spelling: spelling.into(),
        value: literal.value(),
    })
}
