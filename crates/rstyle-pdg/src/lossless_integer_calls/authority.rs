use super::*;
use crate::data_library::{Callee, Registry};

pub(super) const STABLE_CORE: &str = "rstyle.lossless-integer.core-stable-crate-id";
pub(super) const SOURCES: &[(&str, &str)] = &[
    (
        "convert/num.rs",
        "sha256:8dbf6c64681e2dee556e0844eef3e53d62a40df0280d25f469352870a401e034",
    ),
    (
        "num/error.rs",
        "sha256:7447f99e89ce2f0163c83cbe177008c3b789b8129602099dfe9dc3f02909baff",
    ),
    (
        "result.rs",
        "sha256:777e0f516deba3899c8f16c5511da3ebe7debe23f4bcf6a6353c98a927c0f7d1",
    ),
];

pub(super) fn path(actual: &str, expected: &str) -> bool {
    actual == expected || actual == expected.replace("std::", "core::")
}

pub(super) fn verify(
    node: &HirNode,
    kind: Kind,
    facts: &dyn EvidenceFacts,
) -> Result<&'static Registry, String> {
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Unpinned lossless conversion compiler/target")?;
    crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "lossless conversion")?;
    target(&facts.build.pointer_width, registry)?;

    let callee = crate::data_library::callee(node)?;
    definition(&callee, kind.path(), registry)?;
    match (kind, &callee.implementation) {
        (Kind::Convert, Some(implementation)) if implementation.implementation.is_none() => {
            definition(implementation, IMPLEMENTATION, registry)?;
        }
        // Original call-instances independently proves the selected impl. The
        // legacy callee descriptor may contain only the requested trait method.
        (_, None) => {}
        _ => return Err("Lossless conversion selected implementation disagrees".into()),
    }
    Ok(registry)
}

pub(super) fn target(width: &crate::CountFact, registry: &Registry) -> Result<(), String> {
    if width.status != crate::FactStatus::Known
        || width.value != Some(64)
        || registry.pointer_width != 64
    {
        return Err("Lossless conversion requires the reviewed 64-bit target".into());
    }
    sources(registry)
}

pub(super) fn sources(registry: &Registry) -> Result<(), String> {
    // Source pins are reviewed semantic contracts, not routine byte checks.
    // In this exact source, the 64-bit ptr impl uses impl_try_from_unbounded!,
    // whose body is Ok(value as Self); no error value is ever constructed.
    for (file, expected) in SOURCES {
        let actual = registry
            .source_digests
            .get(*file)
            .ok_or_else(|| format!("Lossless conversion lacks reviewed core source {file}"))?;
        if actual != expected {
            return Err(format!(
                "Lossless conversion core source pin differs for {file}"
            ));
        }
    }
    Ok(())
}

fn definition(callee: &Callee, expected: &str, registry: &Registry) -> Result<(), String> {
    if !path(&callee.path, expected)
        || callee.crate_name != "core"
        || callee.crate_hash != registry.core_hash
        || registry.definitions.get(expected) != Some(&callee.definition_hash)
    {
        return Err("Lossless conversion disagrees with reviewed core definition".into());
    }
    Ok(())
}
