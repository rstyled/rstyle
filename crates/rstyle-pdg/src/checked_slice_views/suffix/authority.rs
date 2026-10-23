//! Reviewed sealed RangeFrom SliceIndex delegates to (start..saved_len).get.
use super::*;

pub(super) use super::super::authority::method;

pub(super) fn verify(
    node: &HirNode,
    checked: &Checked<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let registry =
        crate::data_library::registry_for(facts).ok_or("Unpinned checked suffix view")?;
    // Reuse the existing reviewed slice source: RangeFrom delegates to Range,
    // whose checked_sub and end bound guard offset/length pointer construction.
    if registry.source_digests.get("slice").map(String::as_str)
        != Some("sha256:062320a86dc6a3d20718bc8fca824bd1f3df0bb8bb0c3d72e101be989cf576bf")
    {
        return Err("Checked suffix get lacks reviewed sealed SliceIndex source authority".into());
    }
    crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "Checked suffix get")?;
    let resolution = node.res.as_ref().ok_or("Missing suffix get definition")?;
    if resolution.kind != "resolved_callee" || resolution.target.is_some() {
        return Err("Checked suffix get requires direct core method resolution".into());
    }
    let callee: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    method(&callee, registry)?;

    range(checked.range, registry)
}

fn range(node: &HirNode, registry: &crate::data_library::Registry) -> Result<(), String> {
    let resolution = node.res.as_ref().ok_or("Missing suffix range definition")?;
    let callee: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    if resolution.kind != "resolved_range_type" || resolution.target.is_some() {
        return Err("Checked suffix get requires direct core range resolution".into());
    }
    range_definition(&callee, registry)
}

pub(super) fn range_definition(
    callee: &crate::data_library::Callee,
    registry: &crate::data_library::Registry,
) -> Result<(), String> {
    super::super::shared::require_exact_core(
        callee,
        registry,
        super::super::shared::CoreContract {
            paths: &["std::ops::RangeFrom", "core::ops::RangeFrom"],
            canonical: "std::ops::RangeFrom",
            error: "Checked suffix get requires the exact core RangeFrom type",
        },
    )
}
