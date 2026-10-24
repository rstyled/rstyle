//! Shared byte-view shape predicates; neither proves source or origin authority.
use super::TypeTree;

pub(crate) fn observed(tree: &TypeTree) -> bool {
    matches!(tree, TypeTree::Shared(inner)
        if matches!(inner.as_ref(), TypeTree::Slice(element)
            if matches!(element.as_ref(), TypeTree::Scalar(token) if token == "u8")))
}

pub(crate) fn explicit_lifetime_source(source: &syn::Type, lifetime: &syn::Lifetime) -> bool {
    matches!(source, syn::Type::Reference(reference)
        if reference.mutability.is_none()
            && reference.lifetime.as_ref() == Some(lifetime)
            && matches!(reference.elem.as_ref(), syn::Type::Slice(slice)
                if matches!(slice.elem.as_ref(), syn::Type::Path(path)
                    if path.qself.is_none() && path.path.is_ident("u8"))))
}
