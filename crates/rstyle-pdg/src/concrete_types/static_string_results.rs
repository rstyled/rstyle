//! Observational string-bearing products with a nominal empty error.
//! This shape grants neither cleanup, value operations nor helper admission.
use crate::{
    concrete_types::{Nominal, TypeTree, closed_payload},
    data_types::{ResultGrammar, TypeToken},
};

pub(crate) const GRAMMAR: ResultGrammar = ResultGrammar {
    success: product,
    error: empty_error,
};

fn product(ty: &TypeToken) -> bool {
    matches!(ty, TypeToken::Struct { .. }) && closed_payload(ty) && crate::data_origins::shared(ty)
}

fn empty_error(ty: &TypeToken) -> Option<&Nominal> {
    match ty {
        TypeToken::Struct { nominal, fields } if fields.is_empty() => Some(nominal),
        _ => None,
    }
}

pub(super) fn observed(ok: &TypeTree, err: &TypeTree) -> bool {
    matches!(ok, TypeTree::Struct { .. })
        && matches!(err, TypeTree::Struct { .. })
        && GRAMMAR.permits(&TypeToken::from_tree(ok), &TypeToken::from_tree(err))
}

/// A bounded observation candidate, never a source or value permission. Used by
/// the compiler capture to resolve a concrete associated error in its owner.
pub fn observed_success(tree: &TypeTree) -> bool {
    matches!(tree, TypeTree::Struct { .. })
        && closed_payload(tree)
        && crate::data_origins::shared(&TypeToken::from_tree(tree))
}
