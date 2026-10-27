//! Flat scalar locals select components of one saved initializer value.
mod source;
#[cfg(test)]
mod tests;

use crate::{BindingFact, HirNode, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.named-error-validator-flow";
pub(crate) const SELECTOR: &str = "data.scalar-tuple-binding-component";
pub(crate) const DECLARATION: &str = "data.scalar-tuple-binding";

pub(crate) struct Entry<'a> {
    pub pattern: &'a HirNode,
    pub binding: &'a BindingFact,
    pub position: usize,
    pub token: &'a str,
}

pub(crate) struct Checked<'a> {
    pub initializer: &'a HirNode,
    pub entries: Vec<Entry<'a>>,
}

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && node.kind == "stmt_let"
        && node
            .children
            .first()
            .is_some_and(|pat| pat.kind == "pat_scalar_tuple")
}

pub(crate) fn verify<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Checked<'a>, String> {
    if !candidate(node, facts) {
        return Err("Scalar tuple binding requires its registered local pattern capability".into());
    }
    let [pattern, initializer] = node.children.as_slice() else {
        return Err("Scalar tuple binding requires only a pattern and initializer".into());
    };
    let types = crate::data_aggregates::parsed_type(initializer)?;
    let crate::data_types::TypeToken::Tuple(types) = types else {
        return Err("Scalar tuple binding initializer is not a concrete tuple".into());
    };

    if node.ty.is_some()
        || node.binding.is_some()
        || node.binding_mut.is_some()
        || node.param_index.is_some()
        || pattern.binding.is_some()
        || pattern.binding_mut.is_some()
        || pattern.param_index.is_some()
        || types.is_empty()
        || types.len() > crate::concrete_types::MAX_FIELDS
        || pattern.children.len() != types.len()
        || crate::data_aggregates::token(pattern)? != crate::data_aggregates::token(initializer)?
    {
        return Err(
            "Scalar tuple pattern changes its complete initializer component roster".into(),
        );
    }

    let entries = source::verify(node, pattern, initializer, facts)?;
    if entries.iter().zip(&types).any(|(entry, expected)| {
        crate::data_types::TypeToken::Scalar(entry.token.into()) != *expected
    }) {
        return Err("Tuple binding component changes its exact saved tuple type".into());
    }
    Ok(Checked {
        initializer,
        entries,
    })
}
