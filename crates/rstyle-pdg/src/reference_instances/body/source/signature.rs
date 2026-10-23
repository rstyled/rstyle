//! Source formal and function metadata, independent of projected value displays.
use super::{Flow, Result, Role, nodes};
use crate::ByteSpan;
use syn::spanned::Spanned;

pub(super) fn verify(syntax: &syn::ItemFn, flow: &mut Flow<'_>) -> Result<()> {
    let facts = flow.facts;
    let signature = &facts.signature;
    let span = |syntax| crate::source_evidence::original_span(&facts.source.bytes, syntax);
    let Some(syn::FnArg::Typed(input)) = syntax.sig.inputs.first() else {
        return Err("Constructor source requires one typed formal".into());
    };
    let syn::Pat::Ident(pattern) = &*input.pat else {
        return Err("Constructor source formal requires one binding".into());
    };
    if pattern.ident != "haystack"
        || pattern.by_ref.is_some()
        || pattern.mutability.is_some()
        || syntax.sig.inputs.len() != 1
        || pattern.subpat.is_some()
        || !pattern.attrs.is_empty()
        || !input.attrs.is_empty()
    {
        return Err("Constructor formal source changes binding semantics".into());
    }

    let parameter = &signature.params[0];
    flow.node(
        &parameter.pat,
        pattern,
        nodes::Shape::binding(Role::Formal, Some(0)),
    )?;
    if parameter.position != 0
        || parameter.name_span != span(pattern.ident.span())
        || parameter.ty_span != span(input.ty.span())
        || parameter.symbol != parameter.pat.binding.as_deref().unwrap_or("")
        || parameter.ty.spelling != source(facts, parameter.ty_span)?
        || signature.sig_span
            != (ByteSpan {
                start: 0,
                end: span(syntax.sig.span()).end,
            })
        || signature.has_self
        || signature.receiver.is_some()
        || signature.is_async
        || signature.is_const
        || signature.is_unsafe
        || signature.is_variadic
        || signature.abi.known_value() != Some("Rust")
    {
        return Err(
            "Constructor original signature metadata differs from source/formal binding".into(),
        );
    }
    result(syntax, flow)
}

fn result(syntax: &syn::ItemFn, flow: &Flow<'_>) -> Result<()> {
    let facts = flow.facts;
    let syn::ReturnType::Type(_, ty) = &syntax.sig.output else {
        return Err("Constructor source omits its original result annotation".into());
    };
    let expected = crate::source_evidence::original_span(&facts.source.bytes, ty.span());
    let [generic] = facts.signature.generics.as_slice() else {
        return Err("Constructor original signature omits or invents a method generic".into());
    };
    let Some(syn::GenericParam::Type(parameter)) = syntax.sig.generics.params.first() else {
        return Err("Constructor original method type parameter is absent".into());
    };

    if syntax.sig.generics.params.len() != 1
        || facts.signature.return_ty_span != expected
        || facts.signature.return_ty.spelling != source(facts, expected)?
        || generic.name != "H"
        || generic.span
            != crate::source_evidence::original_span(&facts.source.bytes, parameter.ident.span())
    {
        return Err(
            "Constructor result/generic metadata differs from its exact original source".into(),
        );
    }
    Ok(())
}

fn source(facts: &crate::HirFnFacts, span: ByteSpan) -> Result<&str> {
    crate::source_evidence::byte_range(&facts.source.bytes, span)
}
