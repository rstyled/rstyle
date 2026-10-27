//! Wire leaf decoding only; owner, scope and selected-call authority stay with consumers.
use crate::{ByteSpan, HirNode, HirRef};
use serde_json::Value;

pub(crate) fn decode<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T, String> {
    serde_json::from_value(value.clone()).map_err(|error| error.to_string())
}

pub(crate) fn callbacks<'a>(record: &'a Value, reason: &str) -> Result<&'a [Value], String> {
    match record.get("callbacks") {
        None => Ok(&[]),
        Some(value) => value
            .as_array()
            .map(Vec::as_slice)
            .ok_or_else(|| reason.into()),
    }
}

pub(crate) struct Site {
    hir: Option<HirRef>,
    span: Option<ByteSpan>,
}

impl Site {
    pub(crate) fn read(record: &Value) -> Self {
        Self {
            hir: decode(&record["hir"]).ok(),
            span: decode(&record["span"]).ok(),
        }
    }

    pub(crate) fn matches(&self, node: &HirNode) -> bool {
        self.hir.as_ref() == Some(&node.hir) && self.span == Some(node.span)
    }

    pub(crate) fn collides(&self, node: &HirNode) -> bool {
        self.hir.as_ref() == Some(&node.hir) || self.span == Some(node.span)
    }
}
