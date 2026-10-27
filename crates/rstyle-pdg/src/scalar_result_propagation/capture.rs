//! Supplemental compiler observations; generated HIR remains in the raw tree.
use crate::{ByteSpan, HirRef};
use serde::{Deserialize, Serialize};

pub const IDENTITY: &str = "rstyle.rust.scalar-result-propagation/1";
pub const RESOLUTION: &str = "resolved_scalar_result_propagation";

pub use crate::result_propagation::capture::{Arm, Selection};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub input: String,
    pub output: String,
    pub payload: String,
    pub output_payload: String,
    pub error: String,
    pub residual: String,
    pub control_flow: String,
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
