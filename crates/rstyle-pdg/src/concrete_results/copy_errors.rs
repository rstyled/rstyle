//! Copy fieldless errors retain their actual enum vocabulary and Copy authority.
use crate::{concrete_types::Nominal, data_types::TypeToken, unit_store::EvidenceFacts};

#[cfg(test)]
mod tests;

pub(crate) const CAPABILITY: &str = "rust.copy-fieldless-result-flow";

/// Structural shape only; this grants neither Copy nor source authority.
pub(crate) fn error(token: &TypeToken) -> Option<&Nominal> {
    let TypeToken::FieldlessEnum { nominal, variants } = token else {
        return None;
    };
    let names: std::collections::BTreeSet<_> = variants.iter().map(|(name, _)| name).collect();
    let values: std::collections::BTreeSet<_> = variants.iter().map(|(_, value)| value).collect();
    (!variants.is_empty()
        && variants.len() <= crate::concrete_types::MAX_FIELDS
        && names.len() == variants.len()
        && values.len() == variants.len()
        && names.iter().all(|name| !name.is_empty())
        && nominal
            .source_identity
            .as_ref()
            .is_some_and(|id| !id.is_empty()))
    .then_some(nominal)
}

pub(crate) fn success(token: &TypeToken) -> bool {
    matches!(token, TypeToken::Scalar(value) if value == "usize")
        || crate::data_types::shared_byte_slice(token)
}

pub(crate) const GRAMMAR: crate::data_types::ResultGrammar =
    crate::data_types::ResultGrammar { success, error };

/// Find the bounded Result shape even when an interface hides it in a product.
pub(crate) fn contains_copy_result(token: &TypeToken) -> bool {
    GRAMMAR.contains(token)
}

pub(crate) fn checked_error(token: &TypeToken, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if error(token).is_none()
        || !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || !crate::extraction_profile::facts_support(facts, crate::fieldless_enums::CAPABILITY)
        || facts.build.pointer_width.value != Some(64)
        || !facts.nominal_resolution()?.checked_copy_token(token)
    {
        return Err(
            "Fieldless Result error requires its capability and exact checked Copy source".into(),
        );
    }
    Ok(())
}
