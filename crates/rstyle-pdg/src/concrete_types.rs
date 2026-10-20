//! Bounded, observational concrete type evidence. Never authorizes admission.

mod declarations;
pub mod lifetimes;
mod option_spelling;
mod scalar_enums;
pub use scalar_enums::variant_declarations;
pub mod source_proof;

use crate::source_evidence::byte_offset;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const KEY: &str = "concrete_type_observations";
pub const IDENTITY: &str = "rstyle.rust.concrete-type-capture/1";
pub const MAX_DEPTH: usize = 16;
pub const MAX_ENTRIES: usize = 4096;
pub const MAX_FIELDS: usize = 128;
pub const MAX_SOURCE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "evidence", deny_unknown_fields)]
pub enum TypeTree {
    Scalar(String),
    Tuple(Vec<TypeTree>),
    Array {
        element: Box<TypeTree>,
        length: u64,
    },
    Shared(Box<TypeTree>),
    Slice(Box<TypeTree>),
    Str,
    Option(Box<TypeTree>),
    SliceArrayResult {
        length: u64,
        borrowed: bool,
    },
    Struct {
        nominal: Nominal,
        source: crate::SourceText,
        fields: Vec<Field>,
    },
    Enum {
        nominal: Nominal,
        source: crate::SourceText,
        variants: Vec<Variant>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub position: usize,
    pub name: String,
    pub payload: Field,
}

impl TypeTree {
    pub(crate) fn nominal_parts(&self) -> Option<(&Nominal, &crate::SourceText)> {
        match self {
            Self::Struct {
                nominal, source, ..
            }
            | Self::Enum {
                nominal, source, ..
            } => Some((nominal, source)),
            _ => None,
        }
    }

    pub(crate) fn nominal_fields(&self) -> impl Iterator<Item = &Field> {
        let fields = match self {
            Self::Struct { fields, .. } => Some(fields),
            _ => None,
        };
        let variants = match self {
            Self::Enum { variants, .. } => Some(variants),
            _ => None,
        };
        fields.into_iter().flatten().chain(
            variants
                .into_iter()
                .flatten()
                .map(|variant| &variant.payload),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Nominal {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_identity: Option<String>,
    pub package_name: String,
    pub package_version: String,
    pub manifest_digest: String,
    pub definition_path: String,
    pub declaration_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub position: usize,
    pub name: String,
    pub declaration: crate::SourceText,
    pub ty: TypeTree,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", deny_unknown_fields)]
pub enum Observation {
    Captured { ty: TypeTree },
    Unsupported { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterfaceObservations {
    pub identity: String,
    pub parameters: Vec<Observation>,
    pub result: Observation,
}

impl InterfaceObservations {
    pub fn validate(&self) -> Result<(), String> {
        if self.identity != IDENTITY {
            return Err("Unregistered concrete type observation identity".into());
        }
        let mut entries = 0;
        for observation in self.parameters.iter().chain([&self.result]) {
            match observation {
                Observation::Captured { ty } => check(ty, 0, &mut entries)?,
                Observation::Unsupported { reason } if reason.is_empty() => {
                    return Err("Unsupported concrete type lacks a reason".into());
                }
                Observation::Unsupported { .. } => {}
            }
        }

        Ok(())
    }
}

pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub fn scalar(token: &str) -> bool {
    matches!(
        token,
        "bool"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "unit"
    )
}

pub fn validate(tree: &TypeTree) -> Result<(), String> {
    check(tree, 0, &mut 0)
}

fn check(tree: &TypeTree, depth: usize, entries: &mut usize) -> Result<(), String> {
    *entries += 1;
    if depth > MAX_DEPTH || *entries > MAX_ENTRIES {
        return Err("Concrete type observation exceeds depth/entry budget".into());
    }
    check_shape(tree)?;
    check_children(tree, depth + 1, entries)
}

fn check_shape(tree: &TypeTree) -> Result<(), String> {
    match tree {
        TypeTree::Option(payload) if !option_payload(payload) => {
            Err("Option observation requires a scalar or shared scalar payload".into())
        }
        TypeTree::SliceArrayResult { length, .. } if *length > MAX_FIELDS as u64 => {
            Err("Slice conversion result exceeds supported array length".into())
        }
        TypeTree::Scalar(token) if scalar(token) => Ok(()),
        TypeTree::Scalar(_) => Err("Unsupported scalar in concrete type observation".into()),
        TypeTree::Struct {
            nominal,
            source,
            fields,
        } => check_declaration(nominal, source, fields),
        TypeTree::Enum {
            nominal,
            source,
            variants,
        } => scalar_enums::check(nominal, source, variants),
        _ => Ok(()),
    }
}

fn option_payload(tree: &TypeTree) -> bool {
    match tree {
        TypeTree::Scalar(value) => scalar(value) && value != "unit",
        TypeTree::Shared(value) => {
            matches!(value.as_ref(), TypeTree::Scalar(value) if scalar(value) && value != "unit")
        }
        _ => false,
    }
}

fn check_children(tree: &TypeTree, depth: usize, entries: &mut usize) -> Result<(), String> {
    if tree.nominal_parts().is_some() {
        return check_many(tree.nominal_fields().map(|field| &field.ty), depth, entries);
    }
    match tree {
        TypeTree::Tuple(elements) => check_many(elements.iter(), depth, entries),
        TypeTree::Array { element, .. }
        | TypeTree::Shared(element)
        | TypeTree::Slice(element)
        | TypeTree::Option(element) => check(element, depth, entries),
        _ => Ok(()),
    }
}

fn check_many<'a>(
    types: impl Iterator<Item = &'a TypeTree>,
    depth: usize,
    entries: &mut usize,
) -> Result<(), String> {
    for ty in types {
        check(ty, depth, entries)?;
    }

    Ok(())
}

pub(crate) fn check_declaration_budget(source: &str, fields: &[Field]) -> Result<(), String> {
    if source.len() > MAX_SOURCE_BYTES || fields.len() > MAX_FIELDS {
        return Err("Concrete declaration exceeds source/field budget".into());
    }
    Ok(())
}

fn check_declaration(
    nominal: &Nominal,
    source: &crate::SourceText,
    fields: &[Field],
) -> Result<(), String> {
    check_declaration_budget(source, fields)?;
    check_identity(nominal, source)?;
    let declaration: syn::ItemStruct = syn::parse_str(source)
        .map_err(|error| format!("Invalid concrete struct declaration: {error}"))?;
    if nominal.definition_path.rsplit("::").next() != Some(declaration.ident.to_string().as_str())
        || !declaration.generics.params.is_empty()
            && lifetimes::parameter_count(&declaration.generics).is_err()
        || declaration.fields.len() != fields.len()
    {
        return Err("Concrete struct name, generics or field inventory disagrees".into());
    }
    for (position, (declared, captured)) in declaration.fields.iter().zip(fields).enumerate() {
        check_field(source, position, declared, captured)?;
    }

    Ok(())
}

fn check_identity(nominal: &Nominal, source: &crate::SourceText) -> Result<(), String> {
    if nominal.package_name.is_empty()
        || nominal.package_version.is_empty()
        || !is_digest(&nominal.manifest_digest)
        || nominal.declaration_digest != source.identity()
    {
        return Err(
            "Concrete nominal source/package identity is incomplete or inconsistent".into(),
        );
    }
    Ok(())
}

fn check_field(
    source: &str,
    position: usize,
    declared: &syn::Field,
    captured: &Field,
) -> Result<(), String> {
    use syn::spanned::Spanned;
    let name = declared
        .ident
        .as_ref()
        .map_or_else(|| position.to_string(), ToString::to_string);
    let span = declared.ty.span();
    let start = byte_offset(source, span.start());
    let end = byte_offset(source, span.end());
    if captured.position != position
        || captured.name != name
        || source.get(start..end) != Some(captured.declaration.as_str())
    {
        return Err(
            "Concrete field identity/order/declaration disagrees with original source".into(),
        );
    }

    declarations::check(&declared.ty, &captured.ty)?;

    Ok(())
}

pub fn field_declarations(source: &str) -> Result<Vec<(String, String)>, String> {
    use syn::spanned::Spanned;
    let declaration: syn::ItemStruct = syn::parse_str(source).map_err(|error| error.to_string())?;
    declaration
        .fields
        .iter()
        .enumerate()
        .map(|(position, field)| {
            let name = field
                .ident
                .as_ref()
                .map_or_else(|| position.to_string(), ToString::to_string);
            let span = field.ty.span();
            let spelling = source
                .get(byte_offset(source, span.start())..byte_offset(source, span.end()))
                .ok_or("Missing concrete field source bytes")?
                .to_owned();
            Ok((name, spelling))
        })
        .collect()
}

fn is_digest(value: &str) -> bool {
    let bytes = value.as_bytes();

    bytes.len() == 71
        && &bytes[..7] == b"sha256:"
        && bytes[7..]
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}
