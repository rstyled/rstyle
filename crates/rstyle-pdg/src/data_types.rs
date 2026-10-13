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
    Struct(Nominal),
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
            TypeTree::Struct { nominal, .. } => Self::Struct(nominal.clone()),
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
            Self::Scalar(value) => matches!(value.as_str(), "isize" | "usize"),
            Self::Tuple(values) => values.iter().any(Self::native_width),
            Self::Array { element, .. } | Self::Shared(element) | Self::Slice(element) => {
                element.native_width()
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

fn check(token: &TypeToken, depth: usize, entries: &mut usize) -> Result<(), String> {
    *entries += 1;
    if depth > crate::concrete_types::MAX_DEPTH || *entries > crate::concrete_types::MAX_ENTRIES {
        return Err("Data interface token exceeds recursive budget".into());
    }
    check_contents(token, depth, entries)
}

fn check_contents(token: &TypeToken, depth: usize, entries: &mut usize) -> Result<(), String> {
    match token {
        TypeToken::Scalar(value) if scalar(value) => Ok(()),
        TypeToken::Tuple(values) => {
            for value in values {
                check(value, depth + 1, entries)?;
            }
            Ok(())
        }
        TypeToken::Array { element, length } if *length <= crate::concrete_types::MAX_ARRAY => {
            check(element, depth + 1, entries)
        }
        TypeToken::Shared(value) | TypeToken::Slice(value) => check(value, depth + 1, entries),
        TypeToken::Str => Ok(()),
        // Nominal tokens need their verified declaration/effect boundary.
        _ => Err("Data type token lacks a registered concrete proof".into()),
    }
}
