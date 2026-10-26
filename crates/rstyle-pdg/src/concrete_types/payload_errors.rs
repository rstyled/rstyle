//! Complete error shape observations; no ownership, cleanup or value permission.
use super::{Nominal, TypeTree};
use crate::data_types::{ResultGrammar, TypeToken};

#[cfg(test)]
mod tests;

pub(crate) const GRAMMAR: ResultGrammar = ResultGrammar { success, error };

pub(super) fn observed(ok: &TypeTree, err: &TypeTree) -> bool {
    let success = TypeToken::from_tree(ok);
    if reader_product(&success)
        && !matches!(ok, TypeTree::Tuple(values)
            if matches!(values.as_slice(), [TypeTree::Struct { .. }, _]))
    {
        return false;
    }

    if id_reader(&success) && crate::id_reader_types::product(ok).is_none() {
        return false;
    }
    if id_value(&success) && crate::id_reader_types::original_value(ok).is_none() {
        return false;
    }

    GRAMMAR.permits(&success, &TypeToken::from_tree(err))
}

// This bounded vocabulary is shared by capture validation and canonical tokens.
// It does not select source, cleanup, constructors or helper-call authority.
fn success(token: &TypeToken) -> bool {
    match token {
        TypeToken::Scalar(value) => matches!(value.as_str(), "unit" | "usize"),
        TypeToken::Tuple(values) => {
            reader_product(token)
                || id_reader(token)
                || matches!(values.as_slice(),
            [TypeToken::Scalar(first), TypeToken::Scalar(second)]
            if first == "u32" && second == "usize")
        }
        TypeToken::Struct { .. } => id_value(token),
        _ => false,
    }
}

/// Exactly two scalar tuple wrappers and a saved byte count; no ownership claim.
pub(crate) fn id_reader(token: &TypeToken) -> bool {
    let TypeToken::Tuple(values) = token else {
        return false;
    };
    let [value, TypeToken::Scalar(size)] = values.as_slice() else {
        return false;
    };
    size == "usize" && id_value(value)
}

/// Exact nested ID value vocabulary, without a saved byte count or ownership.
pub(crate) fn id_value(token: &TypeToken) -> bool {
    let TypeToken::Struct { fields, .. } = token else {
        return false;
    };
    let [(outer, TypeToken::Struct { fields, .. })] = fields.as_slice() else {
        return false;
    };
    outer == "0"
        && matches!(fields.as_slice(), [(inner, TypeToken::Scalar(value))]
            if inner == "0" && value == "u32")
}

/// Observational vocabulary only; source, Copy and cleanup need independent proof.
pub(crate) fn reader_product(token: &TypeToken) -> bool {
    matches!(token, TypeToken::Tuple(values)
        if matches!(values.as_slice(), [TypeToken::Struct { fields, .. }, TypeToken::Scalar(size)]
            if size == "usize" && fields.len() == 3
                && fields.iter().all(|(_, field)| matches!(field, TypeToken::Scalar(value) if value == "bool"))))
}

fn error(token: &TypeToken) -> Option<&Nominal> {
    let TypeToken::Struct { nominal, fields } = token else {
        return None;
    };
    let [(name, TypeToken::MixedEnum { variants, .. })] = fields.as_slice() else {
        return None;
    };
    (name == "0" && !variants.is_empty()).then_some(nominal)
}
