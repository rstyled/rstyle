//! Independent source grammar for the actual transparent wrapper size expression.
use syn::{Token, punctuated::Punctuated};

pub(crate) fn member(item: &syn::ImplItemConst) -> Result<(), String> {
    if item.ident != "SIZE"
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || !matches!(&item.ty, syn::Type::Path(ty) if ty.qself.is_none() && ty.path.is_ident("usize"))
    {
        return Err("ID byte width requires one concrete SIZE: usize member".into());
    }
    Ok(())
}

pub(crate) fn implementation(item: &syn::ItemImpl, owner: &str) -> Result<(), String> {
    if item.unsafety.is_some()
        || item.trait_.is_some()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || !matches!(item.self_ty.as_ref(), syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident(owner))
    {
        return Err("ID byte width requires its exact concrete inherent owner".into());
    }
    Ok(())
}

pub(crate) fn original(item: &syn::ImplItemConst, owner: &str) -> Result<(), String> {
    member(item)?;
    let syn::Expr::Call(call) = &item.expr else {
        return Err("ID byte width initializer requires size_of of the actual wrapper".into());
    };
    let syn::Expr::Path(path) = call.func.as_ref() else {
        return Err("ID byte width size_of requires its original core path".into());
    };
    let parts: Vec<_> = path.path.segments.iter().collect();
    if !call.args.is_empty()
        || path.qself.is_some()
        || !path.attrs.is_empty()
        || path.path.leading_colon.is_some()
        || parts.len() != 3
        || parts[0].ident != "core"
        || parts[1].ident != "mem"
        || parts[2].ident != "size_of"
        || !parts[0].arguments.is_empty()
        || !parts[1].arguments.is_empty()
    {
        return Err("ID byte width requires exact core::mem::size_of syntax".into());
    }

    let syn::PathArguments::AngleBracketed(arguments) = &parts[2].arguments else {
        return Err("ID byte width is missing its actual wrapper type argument".into());
    };
    if arguments.args.len() != 1
        || !matches!(&arguments.args[0], syn::GenericArgument::Type(syn::Type::Path(ty)) if ty.qself.is_none() && ty.path.is_ident(owner))
    {
        return Err("ID byte width substituted its original size_of wrapper argument".into());
    }
    Ok(())
}

pub(crate) fn transparent_u32(item: &syn::ItemStruct) -> Result<(), String> {
    let syn::Fields::Unnamed(fields) = &item.fields else {
        return Err("ID size layout requires one original u32 tuple field".into());
    };
    if !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || fields.unnamed.len() != 1
        || !matches!(&fields.unnamed[0].ty, syn::Type::Path(ty) if ty.qself.is_none() && ty.path.is_ident("u32"))
    {
        return Err("ID size layout requires a concrete unary u32 wrapper".into());
    }

    let mut repr = 0;
    for attribute in &item.attrs {
        if attribute.path().is_ident("repr") {
            repr += 1;
            let arguments = attribute
                .parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
                .map_err(|error| error.to_string())?;
            if arguments.len() != 1 || !arguments[0].is_ident("transparent") {
                return Err("ID size layout requires only repr(transparent)".into());
            }
        }
    }
    if repr != 1 {
        return Err("ID size layout requires one original repr(transparent)".into());
    }
    Ok(())
}

pub(crate) fn owner_path(expression: &syn::Expr) -> Result<syn::TypePath, String> {
    let syn::Expr::Path(path) = expression else {
        return Err("ID byte width requires an original associated path".into());
    };
    crate::source_evidence::associated_owner(
        path,
        "SIZE",
        "ID byte width path lost its concrete owner/member",
    )
}
