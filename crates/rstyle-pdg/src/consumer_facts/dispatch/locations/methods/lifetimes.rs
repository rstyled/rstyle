//! Lifetime syntax checks for source declaration identity, without type authority.
use std::collections::BTreeSet;

pub(super) fn parameters(generics: &syn::Generics) -> Option<BTreeSet<String>> {
    if generics.where_clause.is_some() {
        return None;
    }
    let mut names = BTreeSet::new();
    for parameter in &generics.params {
        let syn::GenericParam::Lifetime(parameter) = parameter else {
            return None;
        };
        let name = parameter.lifetime.ident.to_string();
        if !parameter.attrs.is_empty()
            || !parameter.bounds.is_empty()
            || matches!(name.as_str(), "_" | "static")
            || !names.insert(name)
        {
            return None;
        }
    }
    Some(names)
}

pub(super) fn owner_path(owner: &syn::Type) -> Option<&syn::PathSegment> {
    let syn::Type::Path(owner) = owner else {
        return None;
    };
    if owner.qself.is_some() || owner.path.leading_colon.is_some() || owner.path.segments.len() != 1
    {
        return None;
    }
    owner.path.segments.first()
}

pub(super) fn arguments(
    arguments: &syn::PathArguments,
    arity: usize,
    parameters: &BTreeSet<String>,
) -> Option<()> {
    let arguments = match arguments {
        syn::PathArguments::None => return (arity == 0).then_some(()),
        syn::PathArguments::AngleBracketed(arguments) if arguments.colon2_token.is_none() => {
            arguments
        }
        _ => return None,
    };
    if arity == 0 || arguments.args.len() != arity {
        return None;
    }
    for argument in &arguments.args {
        let syn::GenericArgument::Lifetime(lifetime) = argument else {
            return None;
        };
        let name = lifetime.ident.to_string();
        if !matches!(name.as_str(), "_" | "static") && !parameters.contains(&name) {
            return None;
        }
    }
    Some(())
}
