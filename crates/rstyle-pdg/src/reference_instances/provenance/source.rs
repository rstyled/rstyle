//! Exact original impl/method headers from the canonical shared source census.
mod generics;
mod signature;

use super::{
    Result,
    model::{Anchor, Observation, Owner},
};
use crate::{ByteSpan, HirFnFacts, nominal_boundary::census::Census};
use std::sync::Arc;

pub(super) struct Bounds {
    pub owner: usize,
    pub clauses: [Anchor; 2],
}

pub(super) struct Source<'a> {
    pub file: &'a str,
    pub bytes: &'a str,
}

impl Source<'_> {
    pub(super) fn anchor(&self, span: proc_macro2::Span) -> Anchor {
        Anchor {
            source_file: self.file.into(),
            span: crate::source_evidence::original_span(self.bytes, span),
        }
    }
}

pub(super) fn verify(canonical: &HirFnFacts, record: &Observation) -> Result<Bounds> {
    let [parent, method] = record.owners.as_slice() else {
        return Err(
            "Reference provenance requires exactly the original impl/method owner chain".into(),
        );
    };
    if parent.parent.is_some()
        || method.parent.as_ref() != Some(&parent.definition)
        || method.definition != record.definition
        || !parent.definition.same_crate(&record.definition)
    {
        return Err("Reference provenance changes or omits generic-owner ancestry".into());
    }
    let census = canonical.build.frontend_options.data::<Arc<Census>>()?;
    crate::nominal_boundary::census::check_budget(&census)?;
    if census.identity != crate::nominal_boundary::census::IDENTITY {
        return Err("Reference provenance lacks canonical source-census identity".into());
    }
    let file = census
        .files
        .iter()
        .filter(|file| file.path == std::path::Path::new(&parent.declaration.source_file))
        .collect::<Vec<_>>();
    let [file] = file.as_slice() else {
        return Err("Reference owner source is absent or duplicated in canonical census".into());
    };
    let source = Source {
        file: &parent.declaration.source_file,
        bytes: &file.bytes,
    };

    binding(canonical, &source)?;
    let syntax = syn::parse_file(source.bytes).map_err(|error| error.to_string())?;
    let implementation = implementation(&syntax, parent, &source)?;
    let function = method_node(implementation, method, &source)?;
    signature::implementation(implementation)?;
    signature::verify(canonical, record, function, &source)?;
    generics::verify(record, implementation, function, &source)
}

fn binding(canonical: &HirFnFacts, source: &Source<'_>) -> Result<()> {
    if canonical
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        != Some(source.file)
        || source
            .bytes
            .get(canonical.fn_span.start as usize..canonical.fn_span.end as usize)
            != Some(canonical.source.bytes.as_ref())
    {
        return Err("Reference method does not bind the complete canonical original source".into());
    }
    Ok(())
}

fn implementation<'a>(
    syntax: &'a syn::File,
    owner: &Owner,
    source: &Source<'_>,
) -> Result<&'a syn::ItemImpl> {
    let mut found = syntax.items.iter().filter_map(|item| {
        let syn::Item::Impl(item) = item else {
            return None;
        };
        (source.anchor(item.impl_token.span).span.start == owner.declaration.span.start)
            .then_some(item)
    });
    let item = found
        .next()
        .ok_or("Reference original impl anchor is not a source impl")?;
    if found.next().is_some() {
        return Err("Reference source impl anchor is ambiguous".into());
    }

    let expected = Anchor {
        source_file: source.file.into(),
        span: ByteSpan {
            start: source.anchor(item.impl_token.span).span.start,
            end: source.anchor(item.brace_token.span.close()).span.end,
        },
    };
    if owner.declaration != expected || item.unsafety.is_some() || item.defaultness.is_some() {
        return Err("Reference original impl span/qualifiers differ".into());
    }
    Ok(item)
}

fn method_node<'a>(
    implementation: &'a syn::ItemImpl,
    owner: &Owner,
    source: &Source<'_>,
) -> Result<&'a syn::ImplItemFn> {
    let mut found = implementation.items.iter().filter_map(|item| {
        let syn::ImplItem::Fn(item) = item else {
            return None;
        };
        (signature::anchor(item, source) == owner.declaration).then_some(item)
    });
    let item = found
        .next()
        .ok_or("Reference method anchor is not an original impl method")?;
    if found.next().is_some() {
        return Err("Reference original method anchor is ambiguous".into());
    }
    Ok(item)
}
