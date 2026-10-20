//! Bounded checked-access source contracts, separate from graph admission.
//!
//! A verified shape is not a verified graph: conditional payload origins and
//! guarded loads must still be independently established by document checking.

mod operations;
mod patterns;
mod source;
mod types;
pub(crate) use operations::*;

use crate::{HirNode, data_library::Callee, unit_store::EvidenceFacts};

const GET: &str = "core::slice::<impl [T]>::get";

/// Original operands, evaluated once in receiver-then-index order. Constructing
/// the Option does not read scalar contents; only selecting its payload may.
pub struct Get<'a> {
    pub receiver: &'a HirNode,
    pub index: &'a HirNode,
    pub scalar: String,
}

/// Exactly two exhaustive unguarded variants, with a scalar copy under Some.
pub struct Match<'a> {
    pub scrutinee: &'a HirNode,
    pub some_arm: usize,
    pub none_arm: usize,
    pub binding: &'a HirNode,
}

/// Check pinned authority, concrete instantiation and original operand spans.
/// This does not register a call summary or authorize extraction/admission.
pub fn verify_get<'a>(node: &'a HirNode, facts: &dyn EvidenceFacts) -> Result<Get<'a>, String> {
    data_v3(facts)?;
    let [receiver, index] = node.children.as_slice() else {
        return Err("Checked get requires exact receiver and index operands".into());
    };
    if node.kind != "expr_method_call" {
        return Err("Checked get requires an original method invocation".into());
    }
    authority(&crate::data_library::callee(node)?, GET, facts)?;
    source::get(node, facts)?;
    let scalar = types::get(node, facts)?;

    Ok(Get {
        receiver,
        index,
        scalar,
    })
}

/// Verify exhaustive Some(&scalar)/None source shape and resolved core identity.
/// The scrutinee may be a saved Option; this check does not invent its origins.
pub fn verify_match<'a>(node: &'a HirNode, facts: &dyn EvidenceFacts) -> Result<Match<'a>, String> {
    data_v3(facts)?;
    let [scrutinee, first, second] = node.children.as_slice() else {
        return Err("Checked Option match requires exactly two arms".into());
    };
    if node.kind != "expr_match" || node.expansion.is_some() {
        return Err("Checked Option match requires original match syntax".into());
    }
    let scalar = types::payload(scrutinee)?;
    source::matched(node, facts)?;
    let binding = patterns::some(first, &scalar, facts)
        .and_then(|binding| patterns::none(second, &scalar, facts).map(|()| (0, 1, binding)))
        .or_else(|_| {
            patterns::none(first, &scalar, facts)?;
            patterns::some(second, &scalar, facts).map(|binding| (1, 0, binding))
        })?;

    Ok(Match {
        scrutinee,
        some_arm: binding.0,
        none_arm: binding.1,
        binding: binding.2,
    })
}

fn data_v3(facts: &dyn EvidenceFacts) -> Result<(), String> {
    if crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        != crate::extraction_profile::ExtractionProfile::DataV3
    {
        return Err("Checked Option contracts require Data-v3".into());
    }
    Ok(())
}

fn authority(callee: &Callee, expected: &str, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let registry =
        crate::data_library::registry_for(facts).ok_or("Unpinned checked Option build")?;
    // rustc displays the same pinned definitions through either prelude.
    let variant_alias = matches!(
        expected,
        "std::prelude::v1::Some" | "std::prelude::v1::None"
    ) && callee.path.strip_prefix("core::") == expected.strip_prefix("std::");
    if (callee.path != expected && !variant_alias)
        || callee.crate_name != "core"
        || callee.crate_hash != registry.core_hash
        || registry.definitions.get(expected) != Some(&callee.definition_hash)
        || callee.implementation.is_some()
    {
        return Err("Checked Option disagrees with pinned core definition".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn reviewed_checked_access_sources_match_the_pinned_toolchain() {
        let output = std::process::Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let sysroot = String::from_utf8(output.stdout).unwrap();
        let root =
            std::path::Path::new(sysroot.trim()).join("lib/rustlib/src/rust/library/core/src");
        for (file, expected) in [
            (
                "slice/index.rs",
                "sha256:080ad30d5226cc0089ba585248b88118921d19440f9c57c48371bca0d67e219d",
            ),
            (
                "option.rs",
                "sha256:693df7245cf3e669b2c442b62c96c76c43ad1f9cfe79f6bf4306e71c32fa9f5d",
            ),
        ] {
            let source = std::fs::read(root.join(file)).unwrap();
            assert_eq!(crate::concrete_types::digest(&source), expected, "{file}");
        }
    }
}
