//! Fully checked scalar/product operations carry no reference components.
use super::*;

pub(super) fn empty(
    roles: &mut BTreeMap<String, serde_json::Value>,
    node: &PdgNode,
) -> Result<(), String> {
    super::super::guarded_sources::insert_role(
        roles,
        node,
        serde_json::json!({"role":"scalar_or_explicit_empty_variant","origins":[]}),
    )
}
