//! Compare complete normalized value types without interpreting display spellings.
use crate::{HirNode, data_types::TypeToken};

pub(crate) fn verify_node_type(
    node: &HirNode,
    expected: &TypeToken,
    mismatch: &str,
) -> Result<(), String> {
    if &crate::data_types::parse(super::token(node)?)? != expected {
        return Err(mismatch.into());
    }
    Ok(())
}
