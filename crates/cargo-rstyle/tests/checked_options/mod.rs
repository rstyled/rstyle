//! Bounded checked scalar reads and concrete Option pattern evidence.
use super::*;

mod capture_values;
mod composition;
mod contracts;
mod scalar_composition;
mod scalar_values;

fn workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        use core::option::Option::{Some as Present, None as Absent};
        type ByteRef<'a> = core::option::Option<&'a u8>;
        pub fn byte(bytes: &[u8], index: usize) -> u8 {
            match bytes.get(index) { Some(&byte) => byte, None => 0 }
        }
        pub fn alias(value: ByteRef<'_>) -> u8 {
            match value { Present(&byte) => byte, Absent => 0 }
        }
        pub fn mutable(value: core::option::Option<&mut u8>) -> u8 {
            match value { Some(byte) => *byte, None => 0 }
        }
        pub fn nested(value: core::option::Option<core::option::Option<u8>>) -> u8 {
            match value { Some(Some(byte)) => byte, _ => 0 }
        }
        pub mod lookalike {
            pub enum Option { Some(u8), None }
            pub fn read(value: Option) -> u8 {
                match value { Option::Some(byte) => byte, Option::None => 0 }
            }
        }
        pub struct Container { pub byte: u8, pub option: core::option::Option<usize> }
        pub fn owning(value: &Container) -> u8 { value.byte }
        pub fn unrelated(byte: u8) -> u8 { byte }
    "#,
    )
    .unwrap();
    directory
}

#[test]
fn core_option_patterns_retain_payload_bindings_and_resolved_variant_identities() {
    let directory = workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["assessment_complete"], true, "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 7);
    let mut variants = Vec::new();
    for function in functions {
        let facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        assert_eq!(
            function["outcome"],
            if matches!(facts.name.as_str(), "byte" | "unrelated") {
                "admitted"
            } else {
                "excluded"
            },
            "{function}"
        );
        let observations = option_patterns(&facts.body);
        if matches!(facts.name.as_str(), "byte" | "alias") {
            assert_eq!(observations.len(), 2, "{function}");
            assert!(
                facts
                    .bindings
                    .iter()
                    .any(|binding| binding.name == "byte" && !binding.is_param)
            );
            variants.push(check_option_shape(observations));
        } else {
            assert!(observations.is_empty(), "{function}");
            if facts.name == "owning" {
                let observed =
                    &function["facts"]["build"]["frontend_options"]["concrete_type_observations"];
                assert_eq!(observed["parameters"][0]["status"], "Unsupported");
            }
        }
    }
    assert_eq!(variants[0], variants[1]);
}

fn check_option_shape(patterns: Vec<&rstyle_pdg::HirNode>) -> Vec<Option<rstyle_pdg::PathRes>> {
    let some = patterns
        .iter()
        .find(|pattern| pattern.kind == "pat_option_some")
        .unwrap();
    assert_eq!(some.children.len(), 1);
    assert_eq!(some.children[0].kind, "pat_shared");
    let bindings = &some.children[0].children;
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].kind, "pat_binding");
    assert_eq!(
        bindings[0].ty.as_ref().unwrap().normalized.as_deref(),
        Some("u8")
    );
    assert!(bindings[0].binding.is_some());
    for pattern in &patterns {
        let resolution = pattern.res.as_ref().unwrap();
        assert_eq!(resolution.kind, "resolved_option_variant");
        let definition: Value = serde_json::from_str(&resolution.description).unwrap();
        assert_eq!(definition["crate"], "core");
        assert!(!definition["crate_hash"].as_str().unwrap().is_empty());
        assert!(!definition["definition_hash"].as_str().unwrap().is_empty());
    }
    assert_ne!(patterns[0].res, patterns[1].res);
    patterns
        .into_iter()
        .map(|pattern| pattern.res.clone())
        .collect()
}

fn option_patterns(node: &rstyle_pdg::HirNode) -> Vec<&rstyle_pdg::HirNode> {
    let own = matches!(node.kind.as_str(), "pat_option_some" | "pat_option_none").then_some(node);
    own.into_iter()
        .chain(node.children.iter().flat_map(option_patterns))
        .collect()
}

#[test]
fn historical_data_profiles_keep_option_observations_unavailable() {
    for profile in ["hir-data-v1", "hir-data-v2"] {
        let directory = workspace();
        let (output, report) = clones(directory.path(), &["--extraction-profile", profile]);
        assert!(output.status.success(), "{report}");
        for function in report["functions"].as_array().unwrap() {
            let facts: rstyle_pdg::HirFnFacts =
                serde_json::from_value(function["facts"].clone()).unwrap();
            assert!(option_patterns(&facts.body).is_empty(), "{function}");
        }
    }
}
