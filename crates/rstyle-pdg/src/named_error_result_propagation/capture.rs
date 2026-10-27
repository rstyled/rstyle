//! Distinct reader protocol; the retained structural wire layout is shared.
pub const IDENTITY: &str = "rstyle.rust.named-error-reader-propagation/1";
pub const RESOLUTION: &str = "resolved_named_error_reader_propagation";
pub use crate::scalar_result_propagation::capture::{Arm, Selection, Shape, Witness};

/// Separate observed protocol: an exact reader tuple continued into a scalar validator.
pub const VALIDATOR_IDENTITY: &str = "rstyle.rust.named-error-validator-propagation/1";
pub const VALIDATOR_RESOLUTION: &str = "resolved_named_error_validator_propagation";

/// Reader payload continued into a separately proved Copy product result.
pub const PRODUCT_IDENTITY: &str = "rstyle.rust.named-error-product-reader-propagation/1";
pub const PRODUCT_RESOLUTION: &str = "resolved_named_error_product_reader_propagation";

/// A concrete nominal ID continued into an independently proved ID/count reader.
pub const ID_READER_IDENTITY: &str = "rstyle.rust.named-error-id-reader-propagation/1";
pub const ID_READER_RESOLUTION: &str = "resolved_named_error_id_reader_propagation";

/// Separate width-bound reader observation; no wider propagation permission.
pub const U16_IDENTITY: &str = "rstyle.rust.named-error-u16-reader-propagation/1";
pub const U16_RESOLUTION: &str = "resolved_named_error_u16_reader_propagation";
