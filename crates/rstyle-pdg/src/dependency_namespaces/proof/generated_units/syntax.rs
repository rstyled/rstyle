//! A complete structural rule, independent of the invoked macro's spelling.
use super::*;
use proc_macro2::{TokenStream, TokenTree};
use std::ops::Range;

pub(super) const TEMPLATE: &str = include_str!("template.txt");
pub(super) type Coordinates = Vec<(Range<usize>, Range<usize>)>;

pub(super) struct Checked {
    pub header: FileSpan,
    pub templates: Vec<(TemplateRole, FileSpan)>,
}

pub(super) fn items(file: &syn::File) -> Result<(&syn::ItemMacro, &syn::ItemMacro), ProofError> {
    let [
        syn::Item::Use(_),
        syn::Item::Mod(_),
        syn::Item::Macro(_),
        syn::Item::Fn(_),
        syn::Item::Macro(definition),
        syn::Item::Macro(invocation),
    ] = file.items.as_slice()
    else {
        return Err(ProofError::Unsupported(
            "Generated-unit module requires its complete bounded original item roster",
        ));
    };
    if !definition.mac.path.is_ident("macro_rules")
        || definition.ident.is_none()
        || invocation.ident.is_some()
        || !invocation.attrs.is_empty()
        || invocation.mac.path.get_ident() != definition.ident.as_ref()
    {
        return Err(ProofError::Contradictory(
            "Generated-unit invocation lacks its exact preceding local macro binding",
        ));
    }
    if [
        "concat",
        "stringify",
        "stringify_outputs",
        "Debug",
        "Clone",
        "Copy",
    ]
    .iter()
    .any(|name| definition.ident.as_ref().unwrap() == name)
    {
        return Err(ProofError::Unsupported(
            "Generated-unit macro shadows required expansion authority",
        ));
    }
    Ok((definition, invocation))
}

pub(super) fn verify(
    file: &UnitFile,
    definition: &syn::ItemMacro,
    invocation: &syn::ItemMacro,
) -> Result<Checked, ProofError> {
    let end = crate::source_evidence::original_span(&file.bytes, invocation.span()).start as usize;
    let original = file.bytes.get(..end).ok_or(ProofError::Contradictory(
        "Generated-unit invocation leaves its original source",
    ))?;
    let expected: TokenStream = TEMPLATE.parse().map_err(|_| {
        ProofError::Contradictory("Invalid independently specified generated-unit source rule")
    })?;
    let actual: TokenStream = original.parse().map_err(|_| {
        ProofError::Unsupported("Generated-unit prefix is outside the bounded source rule")
    })?;
    let expected_file = syn::parse_file(TEMPLATE).map_err(|_| {
        ProofError::Contradictory("Invalid independently specified generated-unit template")
    })?;
    let syn::Item::Macro(expected_definition) = &expected_file.items[4] else {
        unreachable!()
    };
    let rename = expected_definition
        .ident
        .as_ref()
        .unwrap()
        .span()
        .byte_range();

    let mut comparison = Comparison {
        rename,
        coordinates: Vec::new(),
    };
    comparison.compare(expected, actual, 0)?;
    let mut header = site(file, definition.mac.path.span());
    header.span.end = site(file, definition.ident.as_ref().unwrap().span())
        .span
        .end;

    Ok(Checked {
        header,
        templates: templates::map(file, &comparison.coordinates)?,
    })
}

struct Comparison {
    rename: Range<usize>,
    coordinates: Coordinates,
}

impl Comparison {
    fn compare(
        &mut self,
        expected: TokenStream,
        actual: TokenStream,
        depth: usize,
    ) -> Result<(), ProofError> {
        if depth > 64 || self.coordinates.len() > 32768 {
            return Err(ProofError::Unsupported(
                "Generated-unit token rule exceeds budget",
            ));
        }

        let mut actual = actual.into_iter();
        for expected in expected {
            let actual = actual.next().ok_or(ProofError::Contradictory(
                "Generated-unit source removes required neighboring syntax",
            ))?;
            self.token(&expected, &actual, depth)?;
            self.coordinates
                .push((expected.span().byte_range(), actual.span().byte_range()));
        }

        if actual.next().is_some() {
            return Err(ProofError::Unsupported(
                "Generated-unit source adds unchecked neighboring syntax",
            ));
        }
        Ok(())
    }

    fn token(
        &mut self,
        expected: &TokenTree,
        actual: &TokenTree,
        depth: usize,
    ) -> Result<(), ProofError> {
        let equal = match (expected, actual) {
            (TokenTree::Group(a), TokenTree::Group(b)) if a.delimiter() == b.delimiter() => {
                self.compare(a.stream(), b.stream(), depth + 1)?;
                true
            }
            (TokenTree::Ident(a), TokenTree::Ident(b)) => {
                a == b || a.span().byte_range() == self.rename
            }
            (TokenTree::Punct(a), TokenTree::Punct(b)) => {
                a.as_char() == b.as_char() && a.spacing() == b.spacing()
            }
            (TokenTree::Literal(a), TokenTree::Literal(b)) => a.to_string() == b.to_string(),
            _ => false,
        };
        if !equal {
            return Err(ProofError::Unsupported(
                "Generated-unit full matcher, template, helper or derive syntax differs",
            ));
        }
        Ok(())
    }
}

pub(super) fn root(root: &syn::File, selected: &str) -> Result<(), ProofError> {
    if root
        .attrs
        .iter()
        .any(|attribute| !attribute.path().is_ident("doc") && !attribute.path().is_ident("no_std"))
    {
        return Err(ProofError::Unsupported(
            "Generated-unit producer root has unchecked namespace attributes",
        ));
    }
    let mut found = false;
    for item in &root.items {
        let syn::Item::Mod(module) = item else {
            return Err(ProofError::Unsupported(
                "Generated-unit root allows only original ordinary module declarations",
            ));
        };
        if module.unsafety.is_some()
            || module.content.is_some()
            || module
                .attrs
                .iter()
                .any(|attribute| !attribute.path().is_ident("doc"))
        {
            return Err(ProofError::Unsupported(
                "Generated-unit root has generated, imported or conditional macro scope",
            ));
        }
        if module.ident == selected {
            if found || !matches!(module.vis, syn::Visibility::Public(_)) {
                return Err(ProofError::Contradictory(
                    "Generated-unit module root binding is ambiguous or private",
                ));
            }
            found = true;
        }
    }

    if !found {
        return Err(ProofError::Missing(
            "Missing original public generated-unit module",
        ));
    }
    Ok(())
}
