//! Complete error shape observations; no ownership, cleanup or value permission.
use super::{Nominal, TypeTree};
use crate::data_types::{ResultGrammar, TypeToken};

pub(crate) const GRAMMAR: ResultGrammar = ResultGrammar {
    success: |token| matches!(token, TypeToken::Scalar(value) if value == "usize"),
    error,
};

pub(super) fn observed(ok: &TypeTree, err: &TypeTree) -> bool {
    matches!(ok, TypeTree::Scalar(value) if value == "usize")
        && error(&TypeToken::from_tree(err)).is_some()
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
