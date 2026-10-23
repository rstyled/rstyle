//! Exact unqualified source identifiers; no aliases or expression evaluation.
pub(crate) fn plain_value_path(value: &syn::Expr, name: &str) -> bool {
    matches!(value, syn::Expr::Path(path) if path.attrs.is_empty()
        && path.qself.is_none() && path.path.is_ident(name))
}

pub(crate) fn plain_type_path(value: &syn::Type, name: &str) -> bool {
    matches!(value, syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident(name))
}

#[cfg(test)]
mod tests {
    use super::*;

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
