//! Source-owned typed product constants, without synthetic initializer calls.
mod authority;
mod documents;
mod model;
#[cfg(test)]
mod tests;

use crate::{HirNode, nominal_constants::VerifiedValue, unit_store::EvidenceFacts};
pub use model::{Definition, IDENTITY, KEY, MAX_USES, SelectedUse, Selections};

pub const CAPABILITY: &str = "rust.nominal-product-constants";
pub const CONSTRUCT: &str = "data.product-constant";
pub use documents::verify_documents;

pub fn candidate(node: &HirNode) -> bool {
    node.resolution_kind() == Some(crate::nominal_constants::RESOLUTION)
}

pub fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<VerifiedValue, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Product constant requires its registered capability".into());
    }
    let context = authority::Context::new(node, facts)?;
    let resolution = facts.nominal_resolution()?;
    crate::nominal_boundary::resolution::nominal_constants::verify(
        node,
        facts,
        &resolution,
        &context,
    )
}

/// A canonical complete one-field value, including the independently checked scalar type.
pub fn literal(value: &VerifiedValue) -> Result<String, String> {
    serde_json::to_string(&[(0u8, value.field.ty.as_str(), value.field.value.as_str())])
        .map_err(|error| error.to_string())
}

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        CAPABILITY,
        || facts.body.any(&candidate),
        "Product constant requires its registered document capability",
    )
}
