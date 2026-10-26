//! Original syntax and declaration slots; independent of observed display names.
use super::*;
use syn::{ext::IdentExt, spanned::Spanned};

pub(super) fn signature(function: &syn::ItemFn) -> Result<(), String> {
    let signature = &function.sig;

    if signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
    {
        return Err("Fresh named error requires an ordinary nongeneric Rust function".into());
    }

    Ok(())
}

pub(super) fn static_type(ty: &syn::Type) -> Result<(), String> {
    let syn::Type::Reference(reference) = ty else {
        return Err("Static payload requires an explicit shared reference".into());
    };

    let syn::Type::Path(path) = reference.elem.as_ref() else {
        return Err("Static payload requires builtin str".into());
    };

    if reference.mutability.is_some()
        || reference
            .lifetime
            .as_ref()
            .map(|lifetime| lifetime.ident.to_string())
            .as_deref()
            != Some("static")
        || path.qself.is_some()
        || !path.path.is_ident("str")
    {
        return Err("Static payload requires original &'static str spelling".into());
    }

    Ok(())
}

pub(super) fn variant(syntax: &syn::ExprStruct) -> Result<(syn::Path, String), String> {
    if syntax.qself.is_some() || !syntax.attrs.is_empty() || syntax.rest.is_some() {
        return Err(
            "Fresh named variant cannot carry attributes, qualification or an update base".into(),
        );
    }
    crate::source_evidence::qualified_variant(syn::ExprPath {
        attrs: Vec::new(),
        qself: None,
        path: syntax.path.clone(),
    })
}

pub(super) fn slots(syntax: &syn::ExprStruct, declared: &[&str]) -> Result<Vec<usize>, String> {
    variant(syntax)?;

    if declared.is_empty()
        || syntax.fields.len() != declared.len()
        || declared.len() > crate::concrete_types::MAX_FIELDS
    {
        return Err("Named error requires every original field exactly once".into());
    }

    let mut seen = std::collections::BTreeSet::new();
    syntax
        .fields
        .iter()
        .map(|field| {
            if !field.attrs.is_empty() {
                return Err("Named error fields cannot hide attributed source".into());
            }

            let syn::Member::Named(name) = &field.member else {
                return Err("Named error field requires a named declaration slot".into());
            };

            let name = name.unraw().to_string();

            let position = declared
                .iter()
                .position(|declared| *declared == name)
                .ok_or("Unknown named error field")?;

            if !seen.insert(position) {
                return Err("Named error repeats a field".into());
            }

            Ok(position)
        })
        .collect()
}

pub(super) fn span(
    syntax: &impl Spanned,
    actual: crate::ByteSpan,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let source = &facts.source.bytes;

    let expected = crate::ByteSpan {
        start: crate::source_evidence::byte_offset(source, syntax.span().start()) as u32,
        end: crate::source_evidence::byte_offset(source, syntax.span().end()) as u32,
    };

    if !crate::source_evidence::corresponding_span(source, expected, actual) {
        return Err("Fresh named error source correspondence disagrees".into());
    }

    Ok(())
}

/// Only original builtin u32 spelling or the independently checked static string.
pub(super) fn payload_type(ty: &syn::Type, token: &TypeToken) -> Result<(), String> {
    match token {
        TypeToken::Shared(inner) if **inner == TypeToken::Str => static_type(ty),
        TypeToken::Scalar(name) if name == "u32" => {
            if matches!(ty, syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("u32"))
            {
                Ok(())
            } else {
                Err("Scalar error payload requires original builtin u32 spelling".into())
            }
        }
        _ => Err("Named error field is outside static-string or builtin-u32 payloads".into()),
    }
}
