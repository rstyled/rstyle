//! Exact lifetime-only inherent implementation spelling; no specialization.

pub(crate) fn implementation(item: &syn::ItemImpl) -> bool {
    if item.trait_.is_some() || item.unsafety.is_some() {
        return false;
    }
    let Ok(count) = crate::concrete_types::lifetimes::parameter_count(&item.generics) else {
        return false;
    };
    let syn::Type::Path(owner) = item.self_ty.as_ref() else {
        return false;
    };
    if count == 0 {
        return crate::concrete_types::lifetimes::arguments(owner) == Ok(None);
    }
    if crate::concrete_types::lifetimes::arguments(owner) != Ok(Some(count)) {
        return false;
    }

    let Some(segment) = owner.path.segments.last() else {
        return false;
    };
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return false;
    };
    item.generics
        .params
        .iter()
        .zip(&arguments.args)
        .all(|(parameter, argument)| {
            matches!((parameter, argument),
                (syn::GenericParam::Lifetime(parameter), syn::GenericArgument::Lifetime(argument))
                    if parameter.lifetime.ident == argument.ident)
        })
}

#[cfg(test)]
mod tests {
    use super::implementation;

    #[test]
    fn ordinary_and_exact_alpha_renamed_lifetime_owners_are_supported() {
        for source in [
            "impl Input {}",
            "impl<'h> Input<'h> {}",
            "impl<'renamed> crate::search::Input<'renamed> {}",
            "impl<'a, 'b> Pair<'a, 'b> {}",
            "impl<'left, 'right> Pair<'left, 'right> {}",
        ] {
            let item = syn::parse_str(source).unwrap();
            assert!(implementation(&item), "{source}");
        }
    }

    #[test]
    fn specialized_or_unproved_implementation_lifetimes_are_rejected() {
        for source in [
            "impl<'h> Input<'static> {}",
            "impl Input<'static> {}",
            "impl Input<'_> {}",
            "impl<'h> Input<'_> {}",
            "impl<'h> Input {}",
            "impl<'a, 'b> Pair<'b, 'a> {}",
            "impl<'a, 'b> Pair<'a, 'a> {}",
            "impl<'a, 'b> Pair<'a> {}",
            "impl<'a> Pair<'a, 'a> {}",
            "impl<'a> module::<'a>::Input<'a> {}",
            "impl<'a> <Input<'a> as Trait>::Owner {}",
            "impl<'a> &'a Input<'a> {}",
            "impl<'a: 'static> Input<'a> {}",
            "impl<'a> Input<'a> where 'a: 'static {}",
            "impl Input where Self: Trait {}",
            "impl<T> Input<T> {}",
            "impl<const N: usize> Input<N> {}",
            "impl<'a> Input<u8> {}",
            "impl<'a> Input<1> {}",
            "impl<'a> Trait for Input<'a> {}",
            "unsafe impl<'a> Trait for Input<'a> {}",
        ] {
            let item = syn::parse_str(source).unwrap();
            assert!(!implementation(&item), "{source}");
        }
    }
}
