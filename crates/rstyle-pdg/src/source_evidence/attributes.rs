//! Exact list metadata containing one unqualified identifier.
pub(crate) fn single_identifier_attribute(attribute: &syn::Meta, name: &str, value: &str) -> bool {
    matches!(attribute, syn::Meta::List(list) if list.path.is_ident(name)
        && list.parse_args::<syn::Ident>().is_ok_and(|ident| ident == value))
}

/// Literal documentation only; macro expressions and other attributes carry no permission.
fn literal_documentation(attribute: &syn::Attribute) -> bool {
    matches!(&attribute.meta, syn::Meta::NameValue(value) if value.path.is_ident("doc")
        && matches!(&value.value, syn::Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Str(_))))
}

/// Exactly literal docs plus one bare marker; callers select their bounded grammar.
pub(crate) fn literal_metadata(attributes: &[syn::Attribute], marker: &str) -> bool {
    attributes.iter().all(|attribute| {
        matches!(&attribute.meta, syn::Meta::Path(path) if path.is_ident(marker))
            || literal_documentation(attribute)
    })
}
