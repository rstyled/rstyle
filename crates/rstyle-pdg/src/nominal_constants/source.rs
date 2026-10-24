//! Exact independently selected original declaration and conditional member syntax.
use super::{BuildContext, ProductConstant, SourceSelection, SourceSite, SourceUnit};
use crate::{ByteSpan, conditional_source::Configuration};
use syn::spanned::Spanned;

pub(super) fn verify(
    record: &ProductConstant,
    source: &SourceSelection<'_>,
    build: &BuildContext<'_>,
) -> Result<(), String> {
    implementation(&source.constant_unit, &record.implementation_source, build)?;
    site(
        &source.constant_unit,
        &record.declaration,
        constant_span(source),
    )?;
    let item = source.constant;
    attributes(&item.attrs, build, false)?;
    if !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || item.defaultness.is_some()
        || record.definition.path.rsplit("::").next() != Some(&item.ident.to_string())
        || !same_owner_type(&item.ty, &source.constant_unit.implementation.self_ty)
        || range(source.constant_unit.bytes, item.expr.span()) != record.initializer.span
    {
        return Err("Product constant original declaration or initializer disagrees".into());
    }

    if active_count(source, build)? != 1 || !member_present(source) {
        return Err("Product constant lacks a unique active original member".into());
    }
    Ok(())
}

fn member_present(source: &SourceSelection<'_>) -> bool {
    let span = constant_span(source);
    source
        .constant_unit
        .implementation
        .items
        .iter()
        .any(|item| {
            matches!(item, syn::ImplItem::Const(item)
            if declaration_span(source.constant_unit.bytes, item) == span)
        })
}

pub(super) fn implementation(
    unit: &SourceUnit<'_>,
    expected: &SourceSite,
    build: &BuildContext<'_>,
) -> Result<(), String> {
    let item = unit.implementation;
    attributes(&item.attrs, build, false)?;
    if item.trait_.is_some()
        || item.unsafety.is_some()
        || item.defaultness.is_some()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || owner_name(item).is_none()
    {
        return Err("Product constant requires an original nongeneric inherent impl".into());
    }
    let span = ByteSpan {
        start: range(unit.bytes, item.impl_token.span()).start,
        end: range(unit.bytes, item.brace_token.span.close()).end,
    };

    site(unit, expected, span)
}

pub(super) fn attributes(
    attrs: &[syn::Attribute],
    build: &BuildContext<'_>,
    inline: bool,
) -> Result<(), String> {
    let configuration = Configuration::new(build.cfgs);
    if !configuration.active(attrs)? {
        return Err("Product constant selected original source is inactive".into());
    }
    for attr in configuration.effective_attributes(attrs)? {
        let path = attr.path();
        if !path.is_ident("doc") && !path.is_ident("cfg") && !(inline && path.is_ident("inline")) {
            return Err("Product constant has an unsupported active source attribute".into());
        }
    }
    Ok(())
}

pub(super) fn site(
    unit: &SourceUnit<'_>,
    expected: &SourceSite,
    span: ByteSpan,
) -> Result<(), String> {
    if unit.file.is_empty()
        || unit.file != expected.file
        || span != expected.span
        || span.start >= span.end
    {
        return Err("Product constant original file or source span disagrees".into());
    }
    crate::source_evidence::byte_range(unit.bytes, span)?;
    Ok(())
}

pub(super) fn range(bytes: &str, span: proc_macro2::Span) -> ByteSpan {
    crate::source_evidence::original_span(bytes, span)
}

pub(super) fn owner_name(item: &syn::ItemImpl) -> Option<&syn::Ident> {
    let syn::Type::Path(path) = item.self_ty.as_ref() else {
        return None;
    };
    if path.qself.is_some() || path.path.leading_colon.is_some() || path.path.segments.len() != 1 {
        return None;
    }
    let segment = path.path.segments.first()?;
    matches!(segment.arguments, syn::PathArguments::None).then_some(&segment.ident)
}

pub(super) fn same_owner_type(actual: &syn::Type, expected: &syn::Type) -> bool {
    let (syn::Type::Path(actual), syn::Type::Path(expected)) = (actual, expected) else {
        return false;
    };
    actual.qself.is_none()
        && expected.qself.is_none()
        && actual.path.leading_colon.is_none()
        && expected.path.leading_colon.is_none()
        && actual.path.segments.len() == 1
        && expected.path.segments.len() == 1
        && actual.path.segments[0].arguments.is_empty()
        && expected.path.segments[0].arguments.is_empty()
        && actual.path.segments[0].ident == expected.path.segments[0].ident
}

fn active_count(source: &SourceSelection<'_>, build: &BuildContext<'_>) -> Result<usize, String> {
    let mut count = 0;
    for item in &source.constant_unit.implementation.items {
        if let syn::ImplItem::Const(item) = item
            && item.ident == source.constant.ident
            && Configuration::new(build.cfgs).active(&item.attrs)?
        {
            count += 1;
        }
    }
    Ok(count)
}

fn constant_span(source: &SourceSelection<'_>) -> ByteSpan {
    declaration_span(source.constant_unit.bytes, source.constant)
}

fn declaration_span(bytes: &str, item: &syn::ImplItemConst) -> ByteSpan {
    let start = if matches!(item.vis, syn::Visibility::Inherited) {
        item.const_token.span()
    } else {
        item.vis.span()
    };
    ByteSpan {
        start: range(bytes, start).start,
        end: range(bytes, item.semi_token.span()).end,
    }
}
