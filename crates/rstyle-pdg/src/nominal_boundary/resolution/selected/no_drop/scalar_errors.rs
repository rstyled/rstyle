//! Original named scalar fields prove cleanup absence without granting Copy.
use super::*;
use std::collections::BTreeSet;

pub(in crate::nominal_boundary::resolution::selected) const CAPABILITY: &str =
    "rust.fresh-scalar-error-results";

pub(in crate::nominal_boundary::resolution::selected) fn supported(tree: &TypeTree) -> bool {
    let TypeTree::Struct { fields, .. } = tree else {
        return false;
    };
    if fields.is_empty() || fields.len() > concrete_types::MAX_FIELDS {
        return false;
    }
    let names: BTreeSet<_> = fields.iter().map(|field| &field.name).collect();

    names.len() == fields.len()
        && fields.iter().all(|field| {
            syn::parse_str::<syn::Ident>(&field.name).is_ok()
                && matches!(&field.ty, TypeTree::Scalar(value)
                    if concrete_types::scalar(value) && value != "unit")
        })
}

pub(super) fn declaration(item: &syn::ItemStruct, tree: &TypeTree) -> Result<(), String> {
    if !supported(tree) || !item.generics.params.is_empty() || item.generics.where_clause.is_some()
    {
        return Err("Scalar error cleanup requires a complete nongeneric named declaration".into());
    }
    let (syn::Fields::Named(source), TypeTree::Struct { fields, .. }) = (&item.fields, tree) else {
        return Err("Scalar error cleanup requires original named fields".into());
    };
    if source.named.len() != fields.len() {
        return Err("Scalar error cleanup field inventory disagrees with original source".into());
    }

    for (position, (source, observed)) in source.named.iter().zip(fields).enumerate() {
        let TypeTree::Scalar(token) = &observed.ty else {
            return Err("Scalar error cleanup contains a non-scalar field".into());
        };
        if observed.position != position
            || source
                .ident
                .as_ref()
                .is_none_or(|name| name != &observed.name)
            || observed.declaration.as_str() != token
            || !matches!(&source.ty, syn::Type::Path(path)
                if path.qself.is_none() && path.path.is_ident(token))
        {
            return Err("Scalar error cleanup field name/order/type differs from original".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
