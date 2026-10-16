//! Tested pinned library authority, separate from historical primitive summaries.

mod documents;
mod resolve;
mod verify;
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
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        serde_json::from_str(include_str!("../contracts/read-only-lengths-v2.json"))
            .expect("independently tested read-only library registry")
    })
}

pub fn registries() -> [&'static Registry; 2] {
    static X86_64: OnceLock<Registry> = OnceLock::new();
    let x86_64 = X86_64.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../contracts/read-only-lengths-x86_64-v2.json"
        ))
        .expect("independently tested x86_64 read-only library registry")
    });
    [registry(), x86_64]
}

pub fn registry_for(facts: &crate::HirFnFacts) -> Option<&'static Registry> {
    registries().into_iter().find(|registry| {
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
            .into_iter()
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
        let [arm, x86] = registries();
        assert_ne!(arm.identity, x86.identity);
        assert_ne!(arm.target, x86.target);
        assert_ne!(arm.core_hash, x86.core_hash);
        assert_eq!(arm.rustc_commit, x86.rustc_commit);
        assert_eq!(arm.source_digests, x86.source_digests);
        assert_eq!(arm.operations.len(), x86.operations.len());
        for (left, right) in arm.operations.iter().zip(&x86.operations) {
            assert_eq!(
                (&left.id, &left.path, &left.referent, &left.result),
                (&right.id, &right.path, &right.referent, &right.result)
            );
            assert_ne!(left.definition_hash, right.definition_hash);
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
