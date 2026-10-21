//! Deterministic local data interface vocabulary, distinct from scalar compat.

use crate::concrete_types::{Nominal, TypeTree, scalar};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TypeToken {
    Scalar(String),
    Tuple(Vec<TypeToken>),
    Array {
        element: Box<TypeToken>,
        length: u64,
    },
    Shared(Box<TypeToken>),
    Slice(Box<TypeToken>),
    Str,
    Option(Box<TypeToken>),
    IntConversionResult(String),
    SliceArrayResult {
        length: u64,
        borrowed: bool,
    },
    Struct {
        nominal: Nominal,
        fields: Vec<(String, TypeToken)>,
    },
    Enum {
        nominal: Nominal,
        variants: Vec<(String, TypeToken)>,
    },
}

impl TypeToken {
    pub fn from_tree(tree: &TypeTree) -> Self {
        match tree {
            TypeTree::Scalar(value) => Self::Scalar(value.clone()),
            TypeTree::Tuple(values) => Self::Tuple(values.iter().map(Self::from_tree).collect()),
            TypeTree::Array { element, length } => Self::Array {
                element: Box::new(Self::from_tree(element)),
                length: *length,
            },
            TypeTree::Shared(value) => Self::Shared(Box::new(Self::from_tree(value))),
            TypeTree::Slice(value) => Self::Slice(Box::new(Self::from_tree(value))),
            TypeTree::Str => Self::Str,
            TypeTree::IntConversionResult(value) => Self::IntConversionResult(value.clone()),
            TypeTree::Option(value) => Self::Option(Box::new(Self::from_tree(value))),
            TypeTree::SliceArrayResult { length, borrowed } => Self::SliceArrayResult {
                length: *length,
                borrowed: *borrowed,
            },
            TypeTree::Struct {
                nominal, fields, ..
            } => Self::Struct {
                nominal: nominal.clone(),
                fields: fields
                    .iter()
                    .map(|field| (field.name.clone(), Self::from_tree(&field.ty)))
                    .collect(),
            },
            TypeTree::Enum {
                nominal, variants, ..
            } => Self::Enum {
                nominal: nominal.clone(),
                variants: variants
                    .iter()
                    .map(|variant| (variant.name.clone(), Self::from_tree(&variant.payload.ty)))
                    .collect(),
            },
        }
    }

    pub fn text(&self) -> Result<String, String> {
        if let Self::Scalar(value) = self {
            return Ok(value.clone());
        }
        serde_json::to_string(self).map_err(|error| error.to_string())
    }

    pub fn native_width(&self) -> bool {
        match self {
            Self::Scalar(value) | Self::IntConversionResult(value) => {
                matches!(value.as_str(), "isize" | "usize")
            }
            Self::Tuple(values) => values.iter().any(Self::native_width),
            Self::Array { element, .. }
            | Self::Shared(element)
            | Self::Slice(element)
            | Self::Option(element) => element.native_width(),
            Self::Struct { fields, .. } => fields.iter().any(|(_, field)| field.native_width()),
            Self::Enum { variants, .. } => {
                variants.iter().any(|(_, payload)| payload.native_width())
            }
            _ => false,
        }
    }
}

pub fn token(tree: &TypeTree) -> Result<String, String> {
    crate::concrete_types::validate(tree)?;
    let text = TypeToken::from_tree(tree).text()?;
    if text.len() > crate::concrete_types::MAX_SOURCE_BYTES {
        return Err("Data interface token exceeds byte budget".into());
    }
    Ok(text)
}

pub fn parse(text: &str) -> Result<TypeToken, String> {
    if scalar(text) {
        return Ok(TypeToken::Scalar(text.into()));
    }
    if text.len() > crate::concrete_types::MAX_SOURCE_BYTES {
        return Err("Data interface token exceeds byte budget".into());
    }

    let token: TypeToken = serde_json::from_str(text).map_err(|error| error.to_string())?;
    check(&token, 0, &mut 0)?;
    if token.text()? != text {
        return Err("Noncanonical data interface serialization".into());
    }
    Ok(token)
}

pub(crate) fn parse_for(
    text: &str,
    profile: crate::extraction_profile::ExtractionProfile,
) -> Result<TypeToken, String> {
    let ty = parse(text)?;
    if text.contains("\"kind\":\"enum\"") && !profile.supports("rust.scalar-enum-values") {
        return Err("Enum type requires its registered scalar payload capability".into());
    }
    if text.contains("\"kind\":\"option\"")
        && (!profile.supports("rust.checked-slice-option")
            || scalar_option(&ty) && !profile.supports("rust.scalar-option-values"))
    {
        return Err("Option type requires its registered payload capability".into());
    }

    if text.contains("\"kind\":\"int_conversion_result\"")
        && !profile.supports("rust.bitset-cardinality")
    {
        return Err("Integer Result requires its registered capability".into());
    }
    if text.contains("\"kind\":\"int_conversion_result\",\"value\":\"u16\"")
        && !profile.supports("rust.end-of-input")
    {
        return Err("Narrowing Result requires its registered capability".into());
    }
    Ok(ty)
}

pub(crate) fn scalar_option(ty: &TypeToken) -> bool {
    match ty {
        TypeToken::Option(payload) => matches!(payload.as_ref(), TypeToken::Scalar(_)),
        TypeToken::Tuple(values) => values.iter().any(scalar_option),
        TypeToken::Array { element, .. }
        | TypeToken::Shared(element)
        | TypeToken::Slice(element) => scalar_option(element),
        TypeToken::Struct { fields, .. } => fields.iter().any(|(_, field)| scalar_option(field)),
        _ => false,
    }
}

fn check(token: &TypeToken, depth: usize, entries: &mut usize) -> Result<(), String> {
    *entries += 1;
    if depth > crate::concrete_types::MAX_DEPTH || *entries > crate::concrete_types::MAX_ENTRIES {
        return Err("Data interface token exceeds recursive budget".into());
    }
    check_contents(token, depth, entries)
}

fn check_contents(token: &TypeToken, depth: usize, entries: &mut usize) -> Result<(), String> {
    if let Some(element) = recursive_element(token) {
        return check(element, depth + 1, entries);
    }

    match token {
        TypeToken::Scalar(value) if scalar(value) => Ok(()),
        TypeToken::Tuple(values) => {
            for value in values {
                check(value, depth + 1, entries)?;
            }
            Ok(())
        }
        TypeToken::Str => Ok(()),
        TypeToken::IntConversionResult(value) if matches!(value.as_str(), "usize" | "u16") => {
            Ok(())
        }
        TypeToken::SliceArrayResult { length, .. }
            if *length <= crate::concrete_types::MAX_FIELDS as u64 =>
        {
            Ok(())
        }
        TypeToken::Struct { fields, .. }
        | TypeToken::Enum {
            variants: fields, ..
        } if valid_nominal(token) => check_nominal_fields(fields, depth, entries),
        // Admission additionally requires independently verified nominal source/effects.
        _ => Err("Data type token lacks a registered concrete proof".into()),
    }
}

fn recursive_element(token: &TypeToken) -> Option<&TypeToken> {
    match token {
        TypeToken::Array { element, .. }
        | TypeToken::Shared(element)
        | TypeToken::Slice(element) => Some(element),
        TypeToken::Option(payload) if matches!(payload.as_ref(), TypeToken::Shared(value) if matches!(value.as_ref(), TypeToken::Scalar(scalar) if scalar != "unit" && crate::concrete_types::scalar(scalar))) => {
            Some(payload)
        }
        TypeToken::Option(payload) if matches!(payload.as_ref(), TypeToken::Scalar(scalar) if scalar != "unit" && crate::concrete_types::scalar(scalar)) => {
            Some(payload)
        }
        _ => None,
    }
}

fn check_nominal_fields(
    fields: &[(String, TypeToken)],
    depth: usize,
    entries: &mut usize,
) -> Result<(), String> {
    let mut names = std::collections::BTreeSet::new();
    for (name, field) in fields {
        if name.is_empty() || !names.insert(name) {
            return Err("Invalid nominal field vocabulary".into());
        }
        check(field, depth + 1, entries)?;
    }
    Ok(())
}

fn valid_nominal(token: &TypeToken) -> bool {
    let (nominal, valid) = match token {
        TypeToken::Struct { nominal, fields } => (nominal, fields.len() <= crate::concrete_types::MAX_FIELDS),
        TypeToken::Enum { nominal, variants } => (nominal, !variants.is_empty() && variants.len() <= 16
            && variants.iter().all(|(_, payload)| matches!(payload, TypeToken::Scalar(value) if scalar(value) && value != "unit"))),
        _ => return false,
    };
    valid
        && nominal
            .source_identity
            .as_ref()
            .is_some_and(|id| !id.is_empty())
}
