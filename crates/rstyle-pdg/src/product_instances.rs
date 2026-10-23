//! Checked concrete product instances retain original source and finding evidence.
//! Projection alone grants no admission, helper, or serialized graph authority.
pub(crate) mod activation;
mod conversion;
mod materialize;
mod source;
#[cfg(test)]
mod test_support;

pub use materialize::verify_range_constructor_projection;
pub use materialize::{CheckedProjection, verify as verify_constructor_projection};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanArgument {
    Span,
    RangeUsize,
}
