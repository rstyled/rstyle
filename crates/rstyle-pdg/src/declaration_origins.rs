//! Original compiler-file origins for observational nominal types.
//! These checks bind bytes and locations, not effects or package authority.

use crate::{
    ByteSpan,
    concrete_types::{InterfaceObservations, Nominal, Observation, TypeTree, digest},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const KEY: &str = "concrete_declaration_origins";
pub const IDENTITY: &str = "rstyle.rust.declaration-origin/1";
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub nominal: Nominal,
    pub source_file: String,
    pub file_bytes: String,
    pub file_digest: String,
    pub declaration_span: ByteSpan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origins {
    pub identity: String,
    pub declarations: Vec<Origin>,
}

/// Independently checks a complete, unique declaration inventory against the
/// original file bytes. It does not re-run or compare producer serialization.
pub fn verify(interface: &InterfaceObservations, origins: &Origins) -> Result<(), String> {
    interface.validate()?;
    if origins.identity != IDENTITY
        || serde_json::to_vec(origins)
            .map_err(|e| e.to_string())?
            .len()
            > MAX_BYTES
    {
        return Err("Unregistered or over-budget declaration origins".into());
    }
    let expected = declarations(interface)?;
    if expected.len() != origins.declarations.len() {
        return Err("Concrete declaration origin inventory disagrees".into());
    }
    let mut seen = BTreeMap::new();
    for origin in &origins.declarations {
        let key = &origin.nominal.definition_path;
        let (nominal, source) = expected.get(key).ok_or("Unexpected declaration origin")?;
        if seen.insert(key, ()).is_some() || *nominal != &origin.nominal {
            return Err("Duplicate or inconsistent nominal declaration origin".into());
        }
        check_source(origin, source)?;
    }

    Ok(())
}

fn check_source(origin: &Origin, declaration: &str) -> Result<(), String> {
    let span = origin.declaration_span;
    if origin.source_file.is_empty()
        || origin.file_digest != digest(origin.file_bytes.as_bytes())
        || origin
            .file_bytes
            .get(span.start as usize..span.end as usize)
            != Some(declaration)
    {
        return Err("Declaration origin file identity or original byte range disagrees".into());
    }
    // Parsing the entire file rejects a forged range into a string/comment.
    let source = origin
        .file_bytes
        .strip_prefix('\u{feff}')
        .unwrap_or(&origin.file_bytes);
    let prefix = origin.file_bytes.len() - source.len();
    let file = syn::parse_file(source).map_err(|e| e.to_string())?;
    let mut visitor = StructSpans {
        source,
        prefix: prefix as u32,
        spans: Vec::new(),
    };
    syn::visit::Visit::visit_file(&mut visitor, &file);
    if visitor
        .spans
        .iter()
        .filter(|candidate| **candidate == span)
        .count()
        != 1
    {
        return Err("Declaration origin does not identify an exact source struct item".into());
    }

    Ok(())
}

struct StructSpans<'a> {
    source: &'a str,
    prefix: u32,
    spans: Vec<ByteSpan>,
}

impl<'ast> syn::visit::Visit<'ast> for StructSpans<'_> {
    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        use syn::spanned::Spanned;
        let span = item.span();
        self.spans.push(ByteSpan {
            start: crate::source_evidence::byte_offset(self.source, span.start()) as u32
                + self.prefix,
            end: crate::source_evidence::byte_offset(self.source, span.end()) as u32 + self.prefix,
        });
    }
}

pub fn declarations(
    interface: &InterfaceObservations,
) -> Result<BTreeMap<String, (&Nominal, &str)>, String> {
    let mut entries = BTreeMap::new();
    for observation in interface.parameters.iter().chain([&interface.result]) {
        if let Observation::Captured { ty } = observation {
            collect(ty, &mut entries)?;
        }
    }

    Ok(entries)
}

fn collect<'a>(
    tree: &'a TypeTree,
    entries: &mut BTreeMap<String, (&'a Nominal, &'a str)>,
) -> Result<(), String> {
    match tree {
        TypeTree::Struct {
            nominal,
            source,
            fields,
        } => {
            collect_struct(nominal, source, fields, entries)?;
        }
        TypeTree::Tuple(elements) => {
            for element in elements {
                collect(element, entries)?;
            }
        }
        TypeTree::Shared(element) | TypeTree::Slice(element) | TypeTree::Array { element, .. } => {
            collect(element, entries)?
        }
        _ => {}
    }

    Ok(())
}

fn collect_struct<'a>(
    nominal: &'a Nominal,
    source: &'a str,
    fields: &'a [crate::concrete_types::Field],
    entries: &mut BTreeMap<String, (&'a Nominal, &'a str)>,
) -> Result<(), String> {
    if entries
        .insert(nominal.definition_path.clone(), (nominal, source))
        .is_some_and(|previous| previous != (nominal, source))
    {
        return Err("Conflicting observations for one nominal declaration".into());
    }
    for field in fields {
        collect(&field.ty, entries)?;
    }

    Ok(())
}
