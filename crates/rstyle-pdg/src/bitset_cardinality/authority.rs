use super::*;

pub(super) fn verify(node: &HirNode, kind: Kind, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let registry =
        crate::data_library::registry_for(facts).ok_or("Unpinned integer operation build")?;
    if facts.build.pointer_width.status != crate::FactStatus::Known
        || facts.build.pointer_width.value != Some(64)
        || registry.pointer_width != 64
    {
        return Err("Integer conversion requires the reviewed 64-bit target".into());
    }

    let callee = crate::data_library::callee(node)?;
    definition(&callee, kind.path(), registry)?;
    match (kind, &callee.implementation) {
        (Kind::Convert, Some(implementation)) => {
            definition(implementation, IMPLEMENTATION, registry)?;
            if implementation.implementation.is_some() {
                return Err("Unexpected nested integer implementation".into());
            }
        }
        (Kind::Convert, None) => {
            return Err("Missing selected integer TryFrom implementation".into());
        }
        (_, Some(_)) => return Err("Unexpected integer operation implementation".into()),
        (_, None) => {}
    }
    Ok(())
}

fn definition(
    callee: &crate::data_library::Callee,
    path: &str,
    registry: &crate::data_library::Registry,
) -> Result<(), String> {
    if callee.crate_name != "core"
        || callee.crate_hash != registry.core_hash
        || callee.path.replace("std::", "core::") != path.replace("std::", "core::")
        || registry.definitions.get(path) != Some(&callee.definition_hash)
    {
        return Err("Integer operation disagrees with reviewed core definition".into());
    }
    Ok(())
}
