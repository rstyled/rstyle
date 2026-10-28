//! Compiler never-type observations for local inventory, without graph permission.
use crate::{ByteSpan, HirRef};
use serde::{Deserialize, Serialize};

pub const KEY: &str = "consumer_control_types";
pub const IDENTITY: &str = "rstyle.consumer-control-types/1";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observations {
    pub identity: String,
    pub never: Vec<Control>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Control {
    pub hir: HirRef,
    pub span: ByteSpan,
    pub kind: String,
}

pub fn control_kind(kind: &str) -> bool {
    matches!(
        kind,
        "expr_loop"
            | "expr_while"
            | "expr_for"
            | "expr_break"
            | "expr_continue"
            | "expr_if"
            | "expr_block"
    )
}
