//! Guarded component origins for complete checked digit-product functions.
pub(crate) mod documents;
mod records;
mod source;

use super::activation::{self, Checked};
use crate::{
    HirNode,
    data_origins::Origin,
    producer::{Pair, PairView},
    unit_store::EvidenceFacts,
};

pub(crate) use source::{source, verify_source_value};

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    activation::candidate(facts)
}
