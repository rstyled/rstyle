//! Bounded source signatures keep the original early lifetime relationship.
use super::{Result, Source};
use crate::reference_instances::provenance::model::{Anchor, Observation};
use crate::{ByteSpan, HirFnFacts};
use syn::spanned::Spanned;

pub(super) fn anchor(function: &syn::ImplItemFn, source: &Source<'_>) -> Anchor {
    let mut anchor = source.anchor(function.sig.span());
    if !matches!(function.vis, syn::Visibility::Inherited) {
        anchor.span.start = source.anchor(function.vis.span()).span.start;
    }
    anchor
}

pub(super) fn verify(
    canonical: &HirFnFacts,
    record: &Observation,
    function: &syn::ImplItemFn,
    source: &Source<'_>,
) -> Result<()> {
    let signature = &function.sig;
    let from = signature.ident == "from";
    if from != record.owners[0].definition.path.starts_with('<') {
        return Err("Reference source method changes inherent/trait owner kind".into());
    }
    let expected = if from {
        "<util::search::Input<'h> as core::convert::From<&'h H>>::from"
    } else {
        "util::search::Input::<'h>::new"
    };
    if canonical.name != expected
        || signature.ident != if from { "from" } else { "new" }
        || signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || signature.inputs.len() != 1
        || record.signature.anchor != anchor(function, source)
        || canonical.fn_span
            != (ByteSpan {
                start: record.signature.anchor.span.start,
                end: source
                    .anchor(function.block.brace_token.span.close())
                    .span
                    .end,
            })
    {
        return Err(
            "Reference original method signature/qualifiers differ from its bounded family".into(),
        );
    }

    input(signature.inputs.first().ok_or("Missing original input")?)?;
    let syn::ReturnType::Type(_, result) = &signature.output else {
        return Err("Reference original signature lacks its result".into());
    };
    if !(matches!(result.as_ref(), syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("Self")))
    {
        input_type(result)?;
    }
    Ok(())
}

fn input(input: &syn::FnArg) -> Result<()> {
    let syn::FnArg::Typed(input) = input else {
        return Err("Reference constructor may not acquire a self receiver".into());
    };
    if !input.attrs.is_empty() {
        return Err("Reference constructor input attributes are outside this source proof".into());
    }
    shared_h(&input.ty)
}

fn shared_h(ty: &syn::Type) -> Result<()> {
    if !matches!(ty, syn::Type::Reference(reference)
        if reference.mutability.is_none()
        && reference.lifetime.as_ref().is_some_and(|region| region.ident == "h")
        && matches!(reference.elem.as_ref(), syn::Type::Path(path)
            if path.qself.is_none() && path.path.is_ident("H")))
    {
        return Err("Reference source input must preserve exactly &'h H".into());
    }
    Ok(())
}

fn input_type(ty: &syn::Type) -> Result<()> {
    let syn::Type::Path(path) = ty else {
        return Err("Reference original self/result type is not Input<'h>".into());
    };
    let Some(segment) = path.path.segments.first() else {
        return Err("Reference original self/result type lacks a path".into());
    };
    if path.qself.is_some()
        || path.path.leading_colon.is_some()
        || path.path.segments.len() != 1
        || segment.ident != "Input"
    {
        return Err("Reference original self/result type is outside exact Input<'h>".into());
    }

    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err("Reference original Input type lacks its lifetime argument".into());
    };
    if arguments.args.len() != 1
        || !matches!(arguments.args.first(), Some(syn::GenericArgument::Lifetime(region)) if region.ident == "h")
    {
        return Err("Reference original Input type changes the inherited lifetime".into());
    }
    Ok(())
}

pub(super) fn implementation(implementation: &syn::ItemImpl) -> Result<()> {
    input_type(&implementation.self_ty)?;
    let from = implementation.trait_.is_some();
    let Some((negative, path, _)) = &implementation.trait_ else {
        return if from {
            Err("Reference From method lacks its original trait impl".into())
        } else {
            Ok(())
        };
    };
    if !from || negative.is_some() || path.leading_colon.is_some() || path.segments.len() != 1 {
        return Err("Reference owner changes inherent/trait impl kind".into());
    }

    let segment = &path.segments[0];
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err("Reference original From trait lacks its argument".into());
    };
    if segment.ident != "From" || arguments.args.len() != 1 {
        return Err("Reference original From trait changes its complete argument roster".into());
    }

    let Some(syn::GenericArgument::Type(argument)) = arguments.args.first() else {
        return Err("Reference original From trait argument is not a shared input".into());
    };
    shared_h(argument)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_lifetimes_mutability_and_self_arguments_have_independent_expectations() {
        shared_h(&syn::parse_str("&'h H").unwrap()).unwrap();
        for text in ["&H", "&'other H", "&'h mut H", "&'h u8", "&'static H"] {
            assert!(shared_h(&syn::parse_str(text).unwrap()).is_err(), "{text}");
        }
        input_type(&syn::parse_str("Input<'h>").unwrap()).unwrap();
        for text in [
            "Input<'other>",
            "Input<'h, H>",
            "Other<'h>",
            "alias::Input<'h>",
        ] {
            assert!(
                input_type(&syn::parse_str(text).unwrap()).is_err(),
                "{text}"
            );
        }
    }

    #[test]
    fn original_from_impl_requires_one_shared_h_argument() {
        let good = "impl<'h, H: ?Sized + AsRef<[u8]>> From<&'h H> for Input<'h> {}";
        implementation(&syn::parse_str(good).unwrap()).unwrap();
        for text in [
            "impl<'h,H> From<&'h mut H> for Input<'h> {}",
            "impl<'h,H> From<&'other H> for Input<'h> {}",
            "impl<'h,H> From<H> for Input<'h> {}",
            "impl<'h,H> Other<&'h H> for Input<'h> {}",
        ] {
            assert!(
                implementation(&syn::parse_str(text).unwrap()).is_err(),
                "{text}"
            );
        }
    }
}
