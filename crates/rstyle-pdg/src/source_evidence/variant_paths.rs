//! Original enum qualification, shared by values with separate ownership rules.

pub(crate) fn qualified_variant(expression: syn::ExprPath) -> Result<(syn::Path, String), String> {
    if expression.qself.is_some()
        || !expression.attrs.is_empty()
        || expression.path.segments.len() < 2
        || expression
            .path
            .segments
            .iter()
            .any(|part| !matches!(part.arguments, syn::PathArguments::None))
    {
        return Err("Variant value requires an enum-qualified nongeneric original path".into());
    }

    let mut owner = expression.path;
    let variant = owner
        .segments
        .pop()
        .ok_or("Missing qualified variant")?
        .into_value()
        .ident
        .to_string();
    owner.segments.pop_punct();
    Ok((owner, variant))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_qualification_preserves_complete_owner_and_variant() {
        for source in [
            "Error::Missing",
            "crate::errors::Error::Missing",
            "::library::Error::Missing",
        ] {
            let (owner, variant) = qualified_variant(syn::parse_str(source).unwrap()).unwrap();
            assert_eq!(variant, "Missing");
            assert!(!owner.segments.trailing_punct());
            let expected: syn::Path = syn::parse_str(source.rsplit_once("::").unwrap().0).unwrap();
            assert_eq!(
                owner.leading_colon.is_some(),
                expected.leading_colon.is_some()
            );
            assert_eq!(
                owner
                    .segments
                    .iter()
                    .map(|part| part.ident.to_string())
                    .collect::<Vec<_>>(),
                expected
                    .segments
                    .iter()
                    .map(|part| part.ident.to_string())
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn aliases_generic_owners_and_qualified_associated_paths_stay_unproven() {
        for source in [
            "saved",
            "Error::<u8>::Missing",
            "Error::Missing::<u8>",
            "<Owner as Trait>::Missing",
        ] {
            assert!(
                qualified_variant(syn::parse_str(source).unwrap()).is_err(),
                "{source}"
            );
        }
        let mut attributed: syn::ExprPath = syn::parse_str("Error::Missing").unwrap();
        attributed.attrs.push(syn::parse_quote!(#[custom]));
        assert!(qualified_variant(attributed).is_err());
    }
}
