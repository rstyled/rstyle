//! Bounded original syntax only; lexical identity and body semantics stay separate.

pub(crate) struct Paths {
    pub self_type: syn::TypePath,
    pub trait_path: syn::Path,
    pub error_type: syn::TypePath,
    pub result_type: syn::TypePath,
}

pub(crate) fn implementation(
    item: &syn::ItemImpl,
    method: &syn::ImplItemFn,
    input: &str,
) -> Result<Paths, String> {
    let Some((None, trait_path, _)) = &item.trait_ else {
        return Err("TryFrom requires a positive original trait impl".into());
    };
    if item.unsafety.is_some()
        || item.defaultness.is_some()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || !item.attrs.is_empty()
        || item.items.len() != 2
    {
        return Err(
            "TryFrom requires exactly its concrete original method and associated Error".into(),
        );
    }
    trait_argument(trait_path, input)?;
    signature(method, input)?;
    let error = associated_error(item)?;
    let syn::ReturnType::Type(_, result) = &method.sig.output else {
        return Err("TryFrom requires an explicit Result return".into());
    };
    let result_type = path(result)?;
    result_arguments(&result_type)?;

    Ok(Paths {
        self_type: nominal_path(&item.self_ty)?.clone(),
        trait_path: trait_path.clone(),
        error_type: nominal_path(&error.ty)?.clone(),
        result_type,
    })
}

fn associated_error(item: &syn::ItemImpl) -> Result<&syn::ImplItemType, String> {
    let errors: Vec<_> = item
        .items
        .iter()
        .filter_map(|item| match item {
            syn::ImplItem::Type(item) => Some(item),
            _ => None,
        })
        .collect();
    let [error] = errors.as_slice() else {
        return Err("TryFrom requires one original associated Error".into());
    };
    if error.ident != "Error"
        || !error.attrs.is_empty()
        || error.defaultness.is_some()
        || !matches!(error.vis, syn::Visibility::Inherited)
        || !error.generics.params.is_empty()
        || error.generics.where_clause.is_some()
    {
        return Err("TryFrom associated Error changes original concrete declaration".into());
    }

    Ok(error)
}

fn signature(method: &syn::ImplItemFn, scalar: &str) -> Result<(), String> {
    let signature = &method.sig;
    if method.defaultness.is_some()
        || !matches!(method.vis, syn::Visibility::Inherited)
        || !method.attrs.is_empty()
        || signature.ident != "try_from"
        || signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
    {
        return Err("TryFrom requires original safe nongeneric static try_from".into());
    }
    let inputs: Vec<_> = signature.inputs.iter().collect();
    let [syn::FnArg::Typed(input)] = inputs.as_slice() else {
        return Err("TryFrom requires exactly one original value argument".into());
    };
    if !matches!(input.pat.as_ref(), syn::Pat::Ident(binding)
        if binding.by_ref.is_none() && binding.mutability.is_none() && binding.subpat.is_none())
        || !input.attrs.is_empty()
        || !path(&input.ty)?.path.is_ident(scalar)
    {
        return Err(
            "TryFrom original argument must bind its exact builtin unsigned input once".into(),
        );
    }

    Ok(())
}

fn trait_argument(path: &syn::Path, scalar: &str) -> Result<(), String> {
    let last = path.segments.last().ok_or("Empty TryFrom trait path")?;
    let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return Err("TryFrom requires its explicit exact unsigned trait argument".into());
    };
    if last.ident != "TryFrom"
        || arguments.colon2_token.is_some()
        || path
            .segments
            .iter()
            .take(path.segments.len() - 1)
            .any(|part| !part.arguments.is_empty())
        || !matches!(arguments.args.iter().collect::<Vec<_>>().as_slice(),
            [syn::GenericArgument::Type(syn::Type::Path(input))] if input.qself.is_none() && input.path.is_ident(scalar))
    {
        return Err("TryFrom source trait path changes concrete input".into());
    }

    Ok(())
}

pub(crate) fn result_arguments(
    result: &syn::TypePath,
) -> Result<(&syn::TypePath, &syn::TypePath), String> {
    let last = result
        .path
        .segments
        .last()
        .ok_or("Empty TryFrom Result path")?;
    let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return Err("TryFrom requires explicit Result success/error arguments".into());
    };
    if last.ident != "Result"
        || arguments.colon2_token.is_some()
        || result
            .path
            .segments
            .iter()
            .take(result.path.segments.len() - 1)
            .any(|part| !part.arguments.is_empty())
    {
        return Err("TryFrom return is outside original Result syntax".into());
    }
    let arguments: Vec<_> = arguments.args.iter().collect();
    let [
        syn::GenericArgument::Type(ok),
        syn::GenericArgument::Type(error),
    ] = arguments.as_slice()
    else {
        return Err("TryFrom Result requires exactly two original type arguments".into());
    };

    Ok((nominal_path(ok)?, nominal_path(error)?))
}

fn nominal_path(ty: &syn::Type) -> Result<&syn::TypePath, String> {
    let path = plain_path(ty)?;
    if path
        .path
        .segments
        .iter()
        .any(|part| !part.arguments.is_empty())
    {
        return Err("TryFrom requires a nongeneric concrete nominal path".into());
    }

    Ok(path)
}

fn path(ty: &syn::Type) -> Result<syn::TypePath, String> {
    Ok(plain_path(ty)?.clone())
}

fn plain_path(ty: &syn::Type) -> Result<&syn::TypePath, String> {
    let syn::Type::Path(path) = ty else {
        return Err("TryFrom requires original concrete type paths".into());
    };
    if path.qself.is_some() {
        return Err("TryFrom original type path cannot use an unresolved projection".into());
    }

    Ok(path)
}
