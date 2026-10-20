//! Tested pinned library authority, separate from historical primitive summaries.

mod documents;
mod resolve;
mod verify;
pub(crate) use documents::verify_calls;
pub use documents::{evidence, verify_documents};
pub use resolve::resolve;
pub(crate) use resolve::source_resolved;
pub use verify::verify_evidence;

use crate::HirNode;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};

pub const KEY: &str = "data_library_resolution";
pub const IDENTITY: &str = "rstyle.rust.library-resolution/2";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Libraries {
    pub identity: String,
    pub libraries: Vec<Library>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Library {
    pub crate_name: String,
    pub crate_hash: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Callee {
    pub path: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub crate_hash: String,
    pub definition_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<Box<Callee>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub identity: String,
    pub rustc_commit: String,
    pub target: String,
    pub pointer_width: u64,
    pub core_hash: String,
    pub source_digests: BTreeMap<String, String>,
    pub operations: Vec<Operation>,
    #[serde(default)]
    pub definitions: BTreeMap<String, String>,
}

impl Registry {
    /// Check the reviewed core identity. Callers separately constrain allowed
    /// source paths, including any explicitly supported core/std aliases.
    pub(crate) fn exact_core_definition(&self, callee: &Callee, key: &str) -> bool {
        callee.crate_name == "core"
            && callee.crate_hash == self.core_hash
            && self.definitions.get(key) == Some(&callee.definition_hash)
            && callee.implementation.is_none()
    }
}

pub(crate) struct CoreDefinition {
    /// The first path is the registry key; other paths are reviewed aliases.
    pub paths: &'static [&'static str],
    pub unavailable: &'static str,
    pub mismatch: &'static str,
}

pub(crate) fn verify_core_definition(
    callee: &Callee,
    facts: &dyn crate::unit_store::EvidenceFacts,
    expected: CoreDefinition,
) -> Result<(), String> {
    let registry = registry_for(facts).ok_or(expected.unavailable)?;
    let key = expected
        .paths
        .first()
        .ok_or("Missing core definition path contract")?;
    if !expected.paths.contains(&callee.path.as_str())
        || !registry.exact_core_definition(callee, key)
    {
        return Err(expected.mismatch.into());
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub id: String,
    pub path: String,
    pub definition_hash: String,
    pub referent: String,
    pub result: String,
}

pub fn registry() -> &'static Registry {
    &registries()[0]
}

pub fn registries() -> &'static [Registry] {
    static REGISTRIES: OnceLock<Vec<Registry>> = OnceLock::new();
    REGISTRIES.get_or_init(|| {
        [
            include_str!("../contracts/read-only-lengths-v2.json"),
            include_str!("../contracts/read-only-lengths-x86_64-v2.json"),
            include_str!("../contracts/read-only-lengths-macos-aarch64-v2.json"),
        ]
        .into_iter()
        .map(|source| {
            serde_json::from_str(source).expect("independently tested pinned library registry")
        })
        .collect()
    })
}

pub fn registry_for(facts: &crate::HirFnFacts) -> Option<&'static Registry> {
    registries().iter().find(|registry| {
        facts.build.rustc_commit.known_value() == Some(registry.rustc_commit.as_str())
            && facts.build.target_triple.known_value() == Some(registry.target.as_str())
            && facts.build.pointer_width.value == Some(registry.pointer_width)
    })
}

pub fn operation(node: &HirNode) -> Option<&'static Operation> {
    let dispatch = node.dispatch.as_ref()?;
    (node.kind == "expr_method_call" && dispatch.kind == "resolved_method").then_some(())?;
    let authority = callee(node).ok().and_then(|callee| {
        registries()
            .iter()
            .find(|registry| registry.core_hash == callee.crate_hash)
    });
    authority
        .unwrap_or_else(registry)
        .operations
        .iter()
        .find(|op| op.path == dispatch.op)
}

pub fn callee(node: &HirNode) -> Result<Callee, String> {
    let resolution = node
        .res
        .as_ref()
        .filter(|res| res.kind == "resolved_callee" && res.target.is_none())
        .ok_or("Missing resolved library definition")?;
    serde_json::from_str(&resolution.description).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_length_contract_counts_elements_and_utf8_bytes() {
        let empty: &[u16] = &[];
        let values = std::hint::black_box(&[4u16, 8, 16][..]);
        let zero_sized = std::hint::black_box(&[(); 5][..]);
        assert_eq!((empty.len(), empty.is_empty()), (0, true));
        assert_eq!((values.len(), values.is_empty()), (3, false));
        assert_eq!((zero_sized.len(), zero_sized.is_empty()), (5, false));
        let unicode = std::hint::black_box("ƒoo😀");
        assert_eq!((unicode.len(), unicode.is_empty()), (8, false));
        let empty_text = std::hint::black_box("");
        assert_eq!((empty_text.len(), empty_text.is_empty()), (0, true));
    }

    #[test]
    fn target_authorities_share_semantics_but_not_definition_identity() {
        let arm = registry();
        for other in &registries()[1..] {
            assert_ne!(arm.identity, other.identity);
            assert_ne!(arm.target, other.target);
            assert_ne!(arm.core_hash, other.core_hash);
            assert_eq!(arm.rustc_commit, other.rustc_commit);
            assert_eq!(arm.source_digests, other.source_digests);
            assert_eq!(arm.operations.len(), other.operations.len());
            assert_eq!(arm.definitions.len(), 24);
            assert_eq!(
                arm.definitions.keys().collect::<Vec<_>>(),
                other.definitions.keys().collect::<Vec<_>>()
            );
            for (path, hash) in &arm.definitions {
                assert_ne!(Some(hash), other.definitions.get(path));
            }
            for (left, right) in arm.operations.iter().zip(&other.operations) {
                assert_eq!(
                    (&left.id, &left.path, &left.referent, &left.result),
                    (&right.id, &right.path, &right.referent, &right.result)
                );
                assert_ne!(left.definition_hash, right.definition_hash);
            }
        }
    }

    #[test]
    fn registered_semantics_bind_the_reviewed_pinned_core_source_modules() {
        let output = std::process::Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let sysroot = String::from_utf8(output.stdout).unwrap();
        for registry in registries() {
            for (module, expected) in &registry.source_digests {
                let path = std::path::Path::new(sysroot.trim()).join(format!(
                    "lib/rustlib/src/rust/library/core/src/{module}/mod.rs"
                ));
                let source = std::fs::read(path).unwrap();
                assert_eq!(&crate::concrete_types::digest(&source), expected);
            }
        }
    }
}
