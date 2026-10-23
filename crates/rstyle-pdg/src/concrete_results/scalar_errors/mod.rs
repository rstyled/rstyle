//! Fresh scalar products and non-Copy scalar errors; shape grants no authority.
mod boundary;
mod shape;
#[cfg(test)]
mod tests;

use crate::{concrete_types::Nominal, data_types::TypeToken, unit_store::EvidenceFacts};

pub(crate) use boundary::{Boundary, checked_error, owned_error, result_boundary};
#[cfg(test)]
use shape::success;
pub(crate) use shape::{GRAMMAR, contains_result, error};

pub(crate) const CAPABILITY: &str = "rust.fresh-scalar-error-results";
