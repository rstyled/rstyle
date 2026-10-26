//! Distinct reader protocol; the retained structural wire layout is shared.
pub const IDENTITY: &str = "rstyle.rust.named-error-reader-propagation/1";
pub const RESOLUTION: &str = "resolved_named_error_reader_propagation";
pub use crate::scalar_result_propagation::capture::{Arm, Selection, Shape, Witness};
