//! Checked arithmetic trait identity only; implementation effects remain visited.
use super::super::{prelude_traits, source_imports};
use crate::data_library::Registry;
use std::collections::BTreeMap;

const REVIEWED_COMMIT: &str = "5c543b0b8c73c7b72bc8284ced4fb22ead15734d";
const NAMES: &[&str] = &[
    "Add",
    "AddAssign",
    "Div",
    "DivAssign",
    "Mul",
    "MulAssign",
    "Neg",
    "Sub",
    "SubAssign",
];

/// These exact core traits are distinct from core::ops::drop::Drop. The receipt
/// grants neither call support nor purity, Copy, or cleanup of any operand type.
pub(in super::super) struct CheckedTrait {
    _private: (),
}

pub(in super::super) fn resolve(
    path: &syn::Path,
    scopes: &[BTreeMap<String, bool>],
    registry: Option<&Registry>,
) -> Option<CheckedTrait> {
    let registry = registry?;
    if registry.rustc_commit != REVIEWED_COMMIT
        || !prelude_traits::pinned(registry)
        || scopes.iter().any(|scope| {
            scope.contains_key("*") || scope.contains_key("core") || scope.contains_key("std")
        })
    {
        return None;
    }
    let mut original = path.clone();
    let arguments = std::mem::take(&mut original.segments.last_mut()?.arguments);
    let resolved = source_imports::resolve(&original, scopes)?;
    let [root, module, name] = resolved.as_slice() else {
        return None;
    };
    if root != "core"
        || module != "ops"
        || !NAMES.contains(&name.as_str())
        || !checked_arguments(name, &arguments)
    {
        return None;
    }

    Some(CheckedTrait { _private: () })
}

fn checked_arguments(name: &str, arguments: &syn::PathArguments) -> bool {
    match arguments {
        syn::PathArguments::None => true,
        syn::PathArguments::AngleBracketed(arguments) if name != "Neg" => {
            arguments.args.len() == 1
                && matches!(arguments.args.first(), Some(syn::GenericArgument::Type(_)))
        }
        _ => false,
    }
}

#[cfg(test)]
mod source_tests;
#[cfg(test)]
mod tests;
