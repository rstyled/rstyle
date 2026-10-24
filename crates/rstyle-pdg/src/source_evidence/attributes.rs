//! Exact list metadata containing one unqualified identifier.
pub(crate) fn single_identifier_attribute(attribute: &syn::Meta, name: &str, value: &str) -> bool {
    matches!(attribute, syn::Meta::List(list) if list.path.is_ident(name)
        && list.parse_args::<syn::Ident>().is_ok_and(|ident| ident == value))
}
