//! Exact reviewed const-impl forwarding grammar; internal const syntax is retained.
//! Pinned core convert/mod.rs UTF-8 ranges: Into [29260,29691), From [29729,29979).
//! Both include original stability attributes, docs, inline and track_caller metadata.
use crate::source_evidence::{plain_type_path as type_path, plain_value_path as value_path};
use syn::{
    Token,
    parse::{Parse, ParseStream},
};

const INTO: &str = include_str!("review/into.rs");
const FROM: &str = include_str!("review/from.rs");

pub(super) fn verify() -> Result<(), String> {
    sources(INTO, FROM)
}

fn sources(into: &str, from: &str) -> Result<(), String> {
    let into: ConstImpl = syn::parse_str(into).map_err(|error| error.to_string())?;
    let from: ConstImpl = syn::parse_str(from).map_err(|error| error.to_string())?;
    header(&into, &["T", "U"], "Into", "U")?;
    header(&from, &["T"], "From", "T")?;
    let bound = into
        .bound
        .as_ref()
        .ok_or("Reviewed Into lost its const From obligation")?;
    if bound.parameter != "U"
        || !path_argument(&bound.trait_path, "From", "T")
        || from.bound.is_some()
    {
        return Err("Reviewed conversion implementation changes its complete obligations".into());
    }

    into_method(&into.method)?;
    from_method(&from.method)
}

struct ConstImpl {
    attributes: Vec<syn::Attribute>,
    generics: syn::Generics,
    trait_path: syn::Path,
    self_ty: syn::Type,
    bound: Option<ConstBound>,
    method: syn::ImplItemFn,
}

struct ConstBound {
    parameter: syn::Ident,
    trait_path: syn::Path,
}

impl Parse for ConstImpl {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let attributes = input.call(syn::Attribute::parse_outer)?;
        input.parse::<Token![const]>()?;
        input.parse::<Token![impl]>()?;
        let generics = input.parse()?;
        let trait_path = input.parse()?;
        input.parse::<Token![for]>()?;
        let self_ty = input.parse()?;

        let bound = input
            .peek(Token![where])
            .then(|| input.parse())
            .transpose()?;
        let content;
        syn::braced!(content in input);
        let method = content.parse()?;
        if !content.is_empty() || !input.is_empty() {
            return Err(input.error(
                "Reviewed conversion must retain its complete single-method implementation",
            ));
        }
        Ok(Self {
            attributes,
            generics,
            trait_path,
            self_ty,
            bound,
            method,
        })
    }
}

impl Parse for ConstBound {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        input.parse::<Token![where]>()?;
        let parameter = input.parse()?;
        input.parse::<Token![:]>()?;
        let marker;
        syn::bracketed!(marker in input);
        marker.parse::<Token![const]>()?;
        if !marker.is_empty() {
            return Err(marker.error("Unexpected const obligation marker"));
        }

        let trait_path = input.parse()?;
        input.parse::<Token![,]>()?;
        Ok(Self {
            parameter,
            trait_path,
        })
    }
}

fn header(
    value: &ConstImpl,
    parameters: &[&str],
    trait_name: &str,
    argument: &str,
) -> Result<(), String> {
    metadata(&value.attributes)?;
    if value.generics.params.len() != parameters.len() || value.generics.where_clause.is_some()
        || value.generics.params.iter().zip(parameters).any(|(parameter, expected)| {
            !matches!(parameter, syn::GenericParam::Type(parameter) if parameter.ident == *expected
                && parameter.attrs.is_empty() && parameter.bounds.is_empty() && parameter.default.is_none())
        })
        || !path_argument(&value.trait_path, trait_name, argument)
        || !type_path(&value.self_ty, "T")
    {
        return Err("Reviewed conversion impl header changes its generic/trait/self identity".into());
    }
    Ok(())
}

fn metadata(values: &[syn::Attribute]) -> Result<(), String> {
    let [stable, unstable] = values else {
        return Err("Reviewed const conversion lost its complete stability metadata".into());
    };
    attribute_values(
        stable,
        "stable",
        &[("feature", "rust1"), ("since", "1.0.0")],
    )?;
    attribute_values(
        unstable,
        "rustc_const_unstable",
        &[("feature", "const_convert"), ("issue", "143773")],
    )
}

fn attribute_values(
    attribute: &syn::Attribute,
    name: &str,
    expected: &[(&str, &str)],
) -> Result<(), String> {
    let values = attribute
        .parse_args_with(syn::punctuated::Punctuated::<syn::Meta, Token![,]>::parse_terminated)
        .map_err(|error| error.to_string())?;
    if !attribute.path().is_ident(name) || values.len() != expected.len()
        || values.iter().zip(expected).any(|(value, (name, expected))| {
            !matches!(value, syn::Meta::NameValue(value) if value.path.is_ident(*name)
                && matches!(&value.value, syn::Expr::Lit(value) if matches!(&value.lit, syn::Lit::Str(text) if text.value() == *expected)))
        })
    {
        return Err("Reviewed conversion stability metadata changed".into());
    }
    Ok(())
}

fn path_argument(path: &syn::Path, name: &str, argument: &str) -> bool {
    let Some(segment) = path
        .segments
        .first()
        .filter(|_| path.segments.len() == 1 && path.leading_colon.is_none())
    else {
        return false;
    };
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return false;
    };
    segment.ident == name
        && arguments.colon2_token.is_none()
        && arguments.args.len() == 1
        && matches!(arguments.args.first(), Some(syn::GenericArgument::Type(value)) if type_path(value, argument))
}

fn method(value: &syn::ImplItemFn, name: &str, output: &str) -> Result<(), String> {
    let signature = &value.sig;
    if signature.ident != name
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || signature.constness.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || signature.inputs.len() != 1
        || value.defaultness.is_some()
        || !matches!(value.vis, syn::Visibility::Inherited)
        || !matches!(&signature.output, syn::ReturnType::Type(_, ty) if type_path(ty, output))
    {
        return Err("Reviewed conversion method signature changed".into());
    }
    attributes(&value.attrs, name)
}

fn attributes(values: &[syn::Attribute], method: &str) -> Result<(), String> {
    let mut semantic = Vec::new();
    for value in values {
        if value.path().is_ident("doc") {
            if !matches!(&value.meta, syn::Meta::NameValue(value) if matches!(&value.value, syn::Expr::Lit(value) if matches!(value.lit, syn::Lit::Str(_))))
            {
                return Err("Reviewed conversion has a nonliteral documentation attribute".into());
            }
        } else {
            semantic.push(&value.meta);
        }
    }

    let allowed = match (method, semantic.as_slice()) {
        ("into", [syn::Meta::Path(inline), syn::Meta::Path(caller)]) => {
            inline.is_ident("inline") && caller.is_ident("track_caller")
        }
        ("from", [syn::Meta::List(inline)]) => {
            inline.path.is_ident("inline")
                && inline
                    .parse_args::<syn::Ident>()
                    .is_ok_and(|name| name == "always")
        }
        _ => false,
    };
    if !allowed {
        return Err("Reviewed conversion changes its inline/track_caller attributes".into());
    }
    Ok(())
}

fn into_method(value: &syn::ImplItemFn) -> Result<(), String> {
    method(value, "into", "U")?;
    let Some(syn::FnArg::Receiver(receiver)) = value.sig.inputs.first() else {
        return Err("Reviewed Into loses its by-value self receiver".into());
    };
    let [syn::Stmt::Expr(syn::Expr::Call(call), None)] = value.block.stmts.as_slice() else {
        return Err("Reviewed Into has extra statements or a different tail".into());
    };
    let syn::Expr::Path(path) = &*call.func else {
        return Err("Reviewed Into delegate is not a fixed path".into());
    };
    let names = crate::source_evidence::plain_relative_path(&path.path);

    if receiver.reference.is_some()
        || receiver.mutability.is_some()
        || receiver.colon_token.is_some()
        || !receiver.attrs.is_empty()
        || path.qself.is_some()
        || !path.attrs.is_empty()
        || names.as_deref() != Some(&["U".into(), "from".into()])
        || !call.attrs.is_empty()
        || call.args.len() != 1
        || !call
            .args
            .first()
            .is_some_and(|value| value_path(value, "self"))
    {
        return Err("Reviewed Into is not exactly U::from(self)".into());
    }
    Ok(())
}

fn from_method(value: &syn::ImplItemFn) -> Result<(), String> {
    method(value, "from", "T")?;
    let Some(syn::FnArg::Typed(parameter)) = value.sig.inputs.first() else {
        return Err("Reviewed From loses its complete by-value parameter".into());
    };
    let syn::Pat::Ident(binding) = &*parameter.pat else {
        return Err("Reviewed From parameter is not a plain binding".into());
    };
    let [syn::Stmt::Expr(tail, None)] = value.block.stmts.as_slice() else {
        return Err("Reviewed From has extra statements or a different tail".into());
    };

    if !parameter.attrs.is_empty()
        || !type_path(&parameter.ty, "T")
        || binding.ident != "t"
        || !binding.attrs.is_empty()
        || binding.by_ref.is_some()
        || binding.mutability.is_some()
        || binding.subpat.is_some()
        || !value_path(tail, "t")
    {
        return Err("Reviewed From is not exact whole-value identity".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_const_impl_contract_and_effectful_changes() {
        assert!(sources(INTO, FROM).is_ok());
        for changed in [
            INTO.replace("U::from(self)", "{ audit(); U::from(self) }"),
            INTO.replace("U::from(self)", "U::from(other)"),
            INTO.replace("#[track_caller]", "#[cold]"),
            INTO.replace("U: [const] From<T>", "T: [const] From<U>"),
            INTO.replace("[const]", "[const, extra]"),
            INTO.replace("impl<T, U>", "impl<T, U, V>"),
            INTO.replace("issue = \"143773\"", "issue = \"unknown\""),
        ] {
            assert!(sources(&changed, FROM).is_err());
        }
        for changed in [
            FROM.replace("        t\n", "        audit(); t\n"),
            FROM.replace("-> T", "-> U"),
        ] {
            assert!(sources(INTO, &changed).is_err());
        }
    }
}
