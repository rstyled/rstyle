use super::*;

fn source() -> &'static str {
    r#"
    use core::option::Option::{Some as Present, None as Absent};
    pub fn some(n: usize) -> Option<usize> { Some(n) }
    pub fn none() -> Option<usize> { None }
    pub fn renamed(n: usize) -> Option<usize> { Present(n) }
    pub fn renamed_none() -> Option<usize> { Absent }
    pub mod user {
        pub enum Option { Some(usize), None }
        pub fn custom(n: usize) -> Option { Option::Some(n) }
        pub fn custom_none() -> Option { Option::None }
    }
    pub mod functions {
        pub fn Some(n: usize) -> core::option::Option<usize> { core::option::Option::Some(n + 1) }
        pub fn call(n: usize) -> core::option::Option<usize> { Some(n) }
    }
    "#
}

#[test]
fn scalar_option_constructor_capture_retains_pinned_identity_across_ci_targets() {
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let output = Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output()
            .unwrap();
        assert!(output.status.success());
        if !Path::new(String::from_utf8(output.stdout).unwrap().trim()).is_dir() {
            continue;
        }
        for no_std in [false, true] {
            check_target(target, no_std);
        }
    }
}

fn check_target(target: &str, no_std: bool) {
    let directory = clone_workspace();
    let source = format!("{}\n{}", if no_std { "#![no_std]" } else { "" }, source());
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--target", target, "--extraction-profile", "hir-data-v3"],
    );
    assert!(output.status.success(), "{target}: {report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 8);
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    for function in functions {
        assert_eq!(function["outcome"], "excluded", "{function}");
        let facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let value = facts.body.children.last().unwrap();
        match facts.name.as_str() {
            "some" | "renamed" => check_authority(value, &facts, "Some"),
            "none" | "renamed_none" => check_authority(value, &facts, "None"),
            "user::custom" | "user::custom_none" | "functions::call" => {
                assert_ne!(
                    value.res.as_ref().map(|res| res.kind.as_str()),
                    Some("resolved_option_variant")
                );
            }
            "functions::Some" => check_authority(value, &facts, "Some"),
            name => panic!("unexpected function: {name}"),
        }
    }
}

fn check_authority(node: &rstyle_pdg::HirNode, facts: &rstyle_pdg::HirFnFacts, variant: &str) {
    let resolution = node.res.as_ref().unwrap();
    assert_eq!(resolution.kind, "resolved_option_variant");
    assert!(resolution.target.is_none());
    let callee: rstyle_pdg::data_library::Callee =
        serde_json::from_str(&resolution.description).unwrap();
    let registry = rstyle_pdg::data_library::registry_for(facts).unwrap();
    assert_eq!(callee.crate_name, "core");
    assert_eq!(callee.crate_hash, registry.core_hash);
    assert_eq!(
        Some(&callee.definition_hash),
        registry
            .definitions
            .get(&format!("std::prelude::v1::{variant}"))
    );
    assert!(callee.implementation.is_none());
}

#[test]
fn scalar_option_constructor_capture_does_not_extend_historical_profiles() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{report}");
    for function in report["functions"].as_array().unwrap() {
        let facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let value = facts.body.children.last().unwrap();
        assert_ne!(
            value.res.as_ref().map(|res| res.kind.as_str()),
            Some("resolved_option_variant")
        );
        assert_eq!(function["outcome"], "excluded");
    }
}
