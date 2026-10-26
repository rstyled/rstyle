//! Complete error shape observations; no ownership, cleanup or value permission.
use super::{Nominal, TypeTree};
use crate::data_types::{ResultGrammar, TypeToken};

#[cfg(test)]
mod tests;

pub(crate) const GRAMMAR: ResultGrammar = ResultGrammar { success, error };

pub(super) fn observed(ok: &TypeTree, err: &TypeTree) -> bool {
    GRAMMAR.permits(&TypeToken::from_tree(ok), &TypeToken::from_tree(err))
}

// This bounded vocabulary is shared by capture validation and canonical tokens.
// It does not select source, cleanup, constructors or helper-call authority.
fn success(token: &TypeToken) -> bool {
    match token {
        TypeToken::Scalar(value) => matches!(value.as_str(), "unit" | "usize"),
        TypeToken::Tuple(values) => matches!(values.as_slice(),
            [TypeToken::Scalar(first), TypeToken::Scalar(second)]
            if first == "u32" && second == "usize"),
        _ => false,
    }
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
