use super::*;

pub(super) fn verify(node: &HirNode, kind: Kind, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Unpinned checked length compiler/target")?;
    if facts.build.pointer_width.status != crate::FactStatus::Known
        || facts.build.pointer_width.value != Some(64)
        || registry.pointer_width != 64
    {
        return Err("Checked length arithmetic requires its reviewed 64-bit target".into());
    }
    let paths: &[&str] = match kind {
        Kind::Add => &["core::num::<impl usize>::checked_add"],
        Kind::Mul => &["core::num::<impl usize>::checked_mul"],
    };
    crate::data_library::verify_core_definition(
        &crate::data_library::callee(node)?,
        facts,
        crate::data_library::CoreDefinition {
            paths,
            unavailable: "Unpinned checked length compiler/target",
            mismatch: "Checked length operation disagrees with reviewed core definition",
        },
    )
}
