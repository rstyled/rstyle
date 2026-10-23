//! Pinned core get and sealed RangeTo SliceIndex authority, never name-only purity.
use super::*;

pub(super) fn verify(
    node: &HirNode,
    checked: &Checked<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let registry =
        crate::data_library::registry_for(facts).ok_or("Unpinned checked prefix view")?;
    // Reviewed sealed RangeTo SliceIndex delegates to (0..end).get(slice).
    // The pinned slice tree checks end <= len and constructs only view metadata.
    if registry.source_digests.get("slice").map(String::as_str)
        != Some("sha256:062320a86dc6a3d20718bc8fca824bd1f3df0bb8bb0c3d72e101be989cf576bf")
    {
        return Err("Checked prefix get lacks reviewed sealed SliceIndex source authority".into());
    }
    crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "Checked prefix get")?;
    let resolution = node.res.as_ref().ok_or("Missing prefix get definition")?;
    if resolution.kind != "resolved_callee" || resolution.target.is_some() {
        return Err("Checked prefix get requires direct core method resolution".into());
    }
    let callee: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    method(&callee, registry)?;
    let range = checked
        .range
        .res
        .as_ref()
        .ok_or("Missing prefix range definition")?;
    let callee: crate::data_library::Callee =
        serde_json::from_str(&range.description).map_err(|error| error.to_string())?;
    if range.kind != "resolved_range_type"
        || range.target.is_some()
        || !super::shared::exact_core(
            &callee,
            registry,
            &["std::ops::RangeTo", "core::ops::RangeTo"],
            "std::ops::RangeTo",
        )
    {
        return Err("Checked prefix get requires the exact core RangeTo type".into());
    }

    Ok(())
}

pub(super) fn method(
    callee: &crate::data_library::Callee,
    registry: &crate::data_library::Registry,
) -> Result<(), String> {
    super::shared::require_exact_core(
        callee,
        registry,
        super::shared::CoreContract {
            paths: &[GET],
            canonical: GET,
            error: "Checked prefix get disagrees with pinned core method",
        },
    )
}
