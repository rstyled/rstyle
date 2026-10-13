//! Finite lifetime-only nominal spelling checks; no type/const substitution.

use super::TypeTree;
use std::collections::BTreeSet;

pub fn declaration_count(source: &str) -> Result<usize, String> {
    let declaration: syn::ItemStruct = syn::parse_str(source).map_err(|error| error.to_string())?;
    parameter_count(&declaration.generics)
}

pub fn parameter_count(generics: &syn::Generics) -> Result<usize, String> {
    if generics.where_clause.is_some() || generics.params.len() > super::MAX_FIELDS {
        return Err("Nominal lifetimes require bounded parameters and no where clause".into());
    }
    let mut names = BTreeSet::new();
    for parameter in &generics.params {
        let syn::GenericParam::Lifetime(parameter) = parameter else {
            return Err("Nominal type/const parameters remain unavailable".into());
        };
        let name = parameter.lifetime.ident.to_string();
        if !parameter.attrs.is_empty()
            || !parameter.bounds.is_empty()
            || matches!(name.as_str(), "static" | "_")
            || !names.insert(name)
        {
            return Err("Unsupported or duplicate nominal lifetime parameter".into());
        }
    }
    Ok(names.len())
}

pub fn arguments(path: &syn::TypePath) -> Result<Option<usize>, String> {
    let Some(last) = path.path.segments.last() else {
        return Err("Empty nominal path".into());
    };
    if path.qself.is_some()
        || path
            .path
            .segments
            .iter()
            .take(path.path.segments.len() - 1)
            .any(|segment| !matches!(segment.arguments, syn::PathArguments::None))
    {
        return Err("Qualified-self or nonterminal generic arguments are unavailable".into());
    }
    match &last.arguments {
        syn::PathArguments::None => Ok(None),
        syn::PathArguments::AngleBracketed(arguments)
            if arguments.colon2_token.is_none()
                && arguments.args.len() <= super::MAX_FIELDS
                && arguments
                    .args
                    .iter()
                    .all(|argument| matches!(argument, syn::GenericArgument::Lifetime(_))) =>
        {
            Ok(Some(arguments.args.len()))
        }
        _ => Err("Nominal path requires only bounded lifetime arguments".into()),
    }
}

pub fn verify_path(path: &syn::TypePath, tree: &TypeTree) -> Result<(), String> {
    let Some(count) = arguments(path)? else {
        return Ok(());
    };
    let TypeTree::Struct { source, .. } = tree else {
        return Err("Lifetime arguments require an exact nominal declaration".into());
    };
    if count != declaration_count(source)? {
        return Err("Explicit lifetime argument/declaration arity disagrees".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_unbounded_unique_lifetime_parameters_cross_the_boundary() {
        for (source, expected) in [
            ("struct View<'a,'b> { x: &'a &'b i32 }", true),
            ("struct View<T> { x: T }", false),
            ("struct View<const N: usize> { x: [u8; N] }", false),
            ("struct View<'a: 'b,'b> { x: &'a &'b i32 }", false),
            ("struct View<'a> where 'a: 'static { x: &'a i32 }", false),
            ("struct View<'a,'a> { x: &'a i32 }", false),
        ] {
            let declaration: syn::ItemStruct = syn::parse_str(source).unwrap();
            assert_eq!(
                parameter_count(&declaration.generics).is_ok(),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn path_arguments_preserve_explicit_arity_and_reject_type_substitution() {
        for (source, expected) in [
            ("View", Some(None)),
            ("m::View<'a,'_>", Some(Some(2))),
            ("View<u32>", None),
            ("View<8>", None),
            ("m<'a>::View", None),
        ] {
            let path = syn::parse_str(source).unwrap();
            assert_eq!(arguments(&path).ok(), expected, "{source}");
        }
    }
}
