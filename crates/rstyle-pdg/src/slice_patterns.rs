//! Typed compiler observations for bounded byte-slice patterns; no admission proof.
pub(crate) mod arithmetic;
pub(crate) mod graph;
pub(crate) mod locations;
mod receipt;
pub(crate) mod semantics;
pub mod source;
mod types;

#[cfg(test)]
mod tests;

pub use receipt::{Binding, CaptureStatus, Details, Observation};
pub use types::{Adjustment, AdjustmentKind, BindingMode, TypeShape};

pub const CAPABILITY: &str = "rust.slice-prefix-product-results";
pub const IDENTITY: &str = "rstyle.rust.slice-pattern-observation/1";
pub const RESOLUTION: &str = "observed_slice_pattern";
