use crate::{ByteSpan, HirNode, ParamFact, unit_store::EvidenceFacts};
use syn::ext::IdentExt;

pub(super) fn function(facts: &dyn EvidenceFacts) -> Result<syn::ItemFn, String> {
    let function: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;
    if !function.attrs.is_empty()
        || function.sig.asyncness.is_some()
        || function.sig.unsafety.is_some()
        || function.sig.variadic.is_some()
    {
        return Err("Consuming Range requires an ordinary synchronous function".into());
    }
    Ok(function)
}

pub(super) fn formal_name(
    function: &syn::ItemFn,
    formal: &ParamFact,
    facts: &dyn EvidenceFacts,
) -> Result<String, String> {
    let Some(syn::FnArg::Typed(input)) = function.sig.inputs.iter().nth(formal.position as usize)
    else {
        return Err("Missing original owned Range parameter".into());
    };
    let syn::Pat::Ident(pattern) = input.pat.as_ref() else {
        return Err("Range formal source is not a plain binding".into());
    };
    if pattern.by_ref.is_some()
        || pattern.mutability.is_some()
        || pattern.subpat.is_some()
        || !corresponds(pattern.ident.span(), formal.name_span, facts)
    {
        return Err("Range formal source is borrowed, mutable or mismatched".into());
    }
    Ok(pattern.ident.unraw().to_string())
}

pub(super) fn corresponds(
    span: proc_macro2::Span,
    hir: ByteSpan,
    facts: &dyn EvidenceFacts,
) -> bool {
    let syntax = ByteSpan {
        start: crate::source_evidence::byte_offset(&facts.source.bytes, span.start()) as u32,
        end: crate::source_evidence::byte_offset(&facts.source.bytes, span.end()) as u32,
    };
    crate::source_evidence::corresponding_span(&facts.source.bytes, syntax, hir)
}

pub(super) fn node_source(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if node.expansion.is_some()
        || facts
            .source
            .bytes
            .get(node.span.start as usize..node.span.end as usize)
            != Some(node.snippet.as_str())
    {
        return Err("Consuming Range node loses exact original source bytes".into());
    }
    Ok(())
}

pub(super) fn range_type(ty: &syn::Type) -> bool {
    let syn::Type::Path(path) = ty else {
        return false;
    };
    let Some(last) = path.path.segments.last() else {
        return false;
    };
    let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return false;
    };
    if arguments.args.len() != 1 {
        return false;
    }

    let Some(syn::GenericArgument::Type(syn::Type::Path(scalar))) = arguments.args.first() else {
        return false;
    };
    path.qself.is_none()
        && last.ident == "Range"
        && scalar.qself.is_none()
        && scalar.path.is_ident("usize")
}
