//! Exact unqualified source identifiers; no aliases or expression evaluation.
pub(crate) fn plain_value_path(value: &syn::Expr, name: &str) -> bool {
    matches!(value, syn::Expr::Path(path) if path.attrs.is_empty()
        && path.qself.is_none() && path.path.is_ident(name))
}

pub(crate) fn plain_type_path(value: &syn::Type, name: &str) -> bool {
    matches!(value, syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident(name))
}

/// Split an exact concrete associated member path without granting nominal authority.
pub(crate) fn associated_owner(
    path: &syn::ExprPath,
    member: &str,
    error: &str,
) -> Result<syn::TypePath, String> {
    let parts = &path.path.segments;
    if path.qself.is_some()
        || !path.attrs.is_empty()
        || path.path.leading_colon.is_some()
        || parts.len() < 2
        || parts.last().is_none_or(|part| part.ident != member)
        || parts.iter().any(|part| !part.arguments.is_empty())
    {
        return Err(error.into());
    }
    Ok(syn::TypePath {
        qself: None,
        path: syn::Path {
            leading_colon: None,
            segments: parts.iter().take(parts.len() - 1).cloned().collect(),
        },
    })
}

/// Require exactly one eligible original item; inspect all items and propagate cfg errors.
pub(crate) fn unique_active<T>(
    items: impl IntoIterator<Item = T>,
    mut eligible: impl FnMut(T) -> Result<bool, String>,
    error: &str,
) -> Result<(), String> {
    let mut count = 0;
    for item in items {
        if eligible(item)? {
            count += 1;
        }
    }

    if count != 1 {
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_selection_requires_one_match_and_preserves_later_errors() {
        assert_eq!(
            unique_active([false, true, false], Ok, "not unique"),
            Ok(())
        );
        for items in [vec![], vec![false], vec![true, true]] {
            assert_eq!(
                unique_active(items, Ok, "not unique"),
                Err("not unique".into())
            );
        }

        let items = [Ok(true), Ok(true), Err("invalid cfg".into())];
        assert_eq!(
            unique_active(items, |item| item, "not unique"),
            Err("invalid cfg".into())
        );
    }

    #[test]
    fn exact_paths_reject_qualification_parameters_and_value_attributes() {
        assert!(plain_value_path(&syn::parse_str("value").unwrap(), "value"));
        for source in [
            "::value",
            "value::<u8>",
            "<T as Trait>::value",
            "#[allow(dead_code)] value",
            "(value)",
        ] {
            let value = syn::parse_str(source).unwrap();
            assert!(!plain_value_path(&value, "value"), "{source}");
        }
        assert!(plain_type_path(&syn::parse_str("T").unwrap(), "T"));
        for source in ["::T", "T<u8>", "<T as Trait>::Item", "&T", "(T)"] {
            let value = syn::parse_str(source).unwrap();
            assert!(!plain_type_path(&value, "T"), "{source}");
        }
    }
}
