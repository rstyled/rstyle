//! Supplemental compiler observations; generated HIR remains in the raw tree.
use crate::{ByteSpan, HirRef, data_library::Callee};
use serde::{Deserialize, Serialize};

pub const IDENTITY: &str = "rstyle.rust.same-error-result-propagation/1";
pub const RESOLUTION: &str = "resolved_same_error_result_propagation";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub method: Callee,
    pub implementation: Callee,
    pub arguments: Vec<String>,
    pub implementation_arguments: Vec<String>,
    pub input: String,
    pub output: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub result: String,
    pub error: String,
    pub residual: String,
    pub control_flow: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arm {
    pub arm: HirRef,
    pub pattern: HirRef,
    pub binding: HirRef,
    pub usage: HirRef,
    pub variant: Callee,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Witness {
    pub identity: String,
    pub root: HirRef,
    pub operand: HirRef,
    pub try_span: ByteSpan,
    pub operand_span: ByteSpan,
    pub branch_call: HirRef,
    pub residual_call: HirRef,
    pub early_return: HirRef,
    pub shape: Shape,
    pub branch: Selection,
    pub residual: Selection,
    pub conversion: Selection,
    pub break_arm: Arm,
    pub continue_arm: Arm,
}
