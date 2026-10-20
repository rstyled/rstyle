//! Check positive Copy observations, without authorizing nominal admission.
use super::{BuiltinCopy, Origin};
use crate::{ByteSpan, HirFnFacts};
use syn::{Token, punctuated::Punctuated, spanned::Spanned};

pub(super) fn verify(
    facts: &HirFnFacts,
    origin: &Origin,
    observed: &BuiltinCopy,
    attributes: &[syn::Attribute],
) -> Result<(), String> {
    authority(facts, observed)?;
    source(facts, origin, observed, attributes)
}

fn authority(facts: &HirFnFacts, observed: &BuiltinCopy) -> Result<(), String> {
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Copy observation requires pinned compiler/target")?;
    for (definition, key) in [
        (&observed.trait_definition, "core::marker::Copy#trait"),
        (&observed.derive_definition, "core::marker::Copy#derive"),
    ] {
        if definition.crate_name != "core"
            || definition.crate_hash != registry.core_hash
            || !matches!(
                definition.path.as_str(),
                "core::marker::Copy" | "std::marker::Copy"
            )
            || definition.implementation.is_some()
            || registry.definitions.get(key) != Some(&definition.definition_hash)
        {
            return Err("Copy observation disagrees with pinned trait/derive authority".into());
        }
    }
    Ok(())
}

fn source(
    facts: &HirFnFacts,
    origin: &Origin,
    observed: &BuiltinCopy,
    attributes: &[syn::Attribute],
) -> Result<(), String> {
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
    if !configuration.active(attributes)? {
        return Err("Copy observation belongs to an inactive struct".into());
    }

    let effective = configuration.effective_attributes(attributes)?;
    let mut matches = 0;
    for attribute in effective {
        let syn::Meta::List(list) = attribute else {
            continue;
        };
        if !list.path.is_ident("derive") {
            continue;
        }
        let paths = list
            .parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
            .map_err(|error| error.to_string())?;
        for path in paths {
            if path.is_ident("Copy") && token_span(origin, path.span()) == observed.derive_span {
                matches += 1;
            }
        }
    }
    if matches != 1 {
        return Err("Copy observation lacks unique active derive-token ownership".into());
    }
    Ok(())
}

fn token_span(origin: &Origin, span: proc_macro2::Span) -> ByteSpan {
    let bytes = origin
        .file_bytes
        .strip_prefix('\u{feff}')
        .unwrap_or(&origin.file_bytes);
    let prefix = (origin.file_bytes.len() - bytes.len()) as u32;
    ByteSpan {
        start: prefix + crate::source_evidence::byte_offset(bytes, span.start()) as u32,
        end: prefix + crate::source_evidence::byte_offset(bytes, span.end()) as u32,
    }
}
