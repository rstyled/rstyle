//! Direct finite scalar fields only; no recursive ownership or borrowed payloads.
use super::*;
use std::collections::BTreeSet;

pub(crate) const GRAMMAR: crate::data_types::ResultGrammar =
    crate::data_types::ResultGrammar { success, error };

pub(crate) fn success(token: &TypeToken) -> bool {
    scalar_product(token).is_some()
}

pub(crate) fn error(token: &TypeToken) -> Option<&Nominal> {
    let nominal = scalar_product(token)?;
    let TypeToken::Struct { fields, .. } = token else {
        return None;
    };
    fields
        .iter()
        .all(|(name, _)| syn::parse_str::<syn::Ident>(name).is_ok())
        .then_some(nominal)
}

/// Find new Result vocabulary even inside unsupported storage or borrowed shapes.
/// Detection is deliberately independent of ownership/source certification.
pub(crate) fn contains_result(token: &TypeToken) -> bool {
    GRAMMAR.contains(token)
}

fn scalar_product(token: &TypeToken) -> Option<&Nominal> {
    let TypeToken::Struct { nominal, fields } = token else {
        return None;
    };
    if fields.is_empty()
        || fields.len() > crate::concrete_types::MAX_FIELDS
        || nominal
            .source_identity
            .as_ref()
            .is_none_or(|identity| identity.is_empty())
    {
        return None;
    }

    let names: BTreeSet<_> = fields.iter().map(|(name, _)| name).collect();
    let named = names
        .iter()
        .all(|name| syn::parse_str::<syn::Ident>(name).is_ok());
    let tuple = fields
        .iter()
        .enumerate()
        .all(|(position, (name, _))| name == &position.to_string());

    (names.len() == fields.len()
        && (named || tuple)
        && fields.iter().all(|(_, field)| {
            matches!(field, TypeToken::Scalar(value) if value != "unit" && crate::concrete_types::scalar(value))
        }))
    .then_some(nominal)
}
