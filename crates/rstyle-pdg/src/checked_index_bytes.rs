//! Copy byte-array inputs for checked owned Results; source and effects stay separate.
use crate::{HirFnFacts, concrete_types::TypeTree};

pub(crate) const CAPABILITY: &str = "rust.byte-array-checked-results";

pub(crate) fn parameter(facts: &HirFnFacts, tree: &TypeTree) -> bool {
    matches!(tree, TypeTree::Scalar(value)
        if crate::concrete_types::scalar(value) && value != "unit")
        || crate::extraction_profile::facts_support(facts, CAPABILITY)
            && matches!(tree, TypeTree::Array { element, length: 4 }
                if matches!(element.as_ref(), TypeTree::Scalar(value) if value == "u8"))
}
