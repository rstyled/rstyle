//! Typed compiler observations for bounded byte-slice patterns; no admission proof.
mod receipt;
pub mod source;
mod types;

#[cfg(test)]
mod tests;

pub use receipt::{Binding, CaptureStatus, Details, Observation};
pub use types::{Adjustment, AdjustmentKind, BindingMode, TypeShape};

pub const CAPABILITY: &str = "rust.slice-prefix-product-results";
pub const IDENTITY: &str = "rstyle.rust.slice-pattern-observation/1";
pub const RESOLUTION: &str = "observed_slice_pattern";
