//! Expanded methods retain function facts without claiming original ownership.
use super::*;

#[test]
fn generated_inherent_methods_do_not_abort_unrelated_scalar_capture() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub struct Value { byte: u8 }
        macro_rules! methods {
            () => {
                impl Value {
                    pub fn first(&self) -> u8 { self.byte }
                    pub fn second(&self) -> u8 { self.byte }
                }
            }
        }
        methods!();
        impl Value { pub fn ordinary(&self) -> u8 { self.byte } }
        pub fn scalar(byte: u8) -> u8 { byte }
        "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["capture_complete"], true, "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 4, "{report}");
    for function in functions {
        check_expanded_ownership(function);
    }
}

fn check_expanded_ownership(function: &Value) {
    let facts = &function["facts"];
    let name = facts["name"].as_str().unwrap();
    let options = &facts["build"]["frontend_options"];
    let definitions = options["inherent_unit_definitions"]["definitions"]
        .as_array()
        .unwrap();
    assert_eq!(definitions.len(), 1, "{function}");
    if name.ends_with("first") || name.ends_with("second") {
        assert_eq!(function["outcome"], "excluded", "{function}");
        assert!(options.get("inherent_definition").is_none(), "{function}");
        assert!(
            facts["source_diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["construct"] == "macro_generated_function")
        );
    } else if name.ends_with("ordinary") {
        assert_eq!(
            options["inherent_definition"]["definition_identity"],
            definitions[0]["definition_identity"],
            "{function}"
        );
    } else if name == "scalar" {
        assert_eq!(function["outcome"], "admitted", "{function}");
    }
}

#[test]
fn generated_initializers_keep_receiver_observations_without_source_certification() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        std::thread_local! {
            static ID: usize = {
                let bytes = [1u8, 2];
                bytes.len() + COUNTER.fetch_add(1, Ordering::Relaxed)
            };
        }
        pub fn scalar(byte: u8) -> u8 { byte }
        "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["capture_complete"], true, "{report}");
    assert_eq!(report["assessment_complete"], true, "{report}");
    let functions = report["functions"].as_array().unwrap();
    let mut checked_initializer = false;
    for function in functions {
        let facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        if facts.name.ends_with("__rust_std_internal_init_fn") {
            check_initializer_receivers(function, &facts);
            checked_initializer = true;
        } else if facts.name == "scalar" {
            assert_eq!(function["outcome"], "admitted", "{function}");
        }
    }
    assert!(checked_initializer, "{report}");
}

fn check_initializer_receivers(function: &Value, facts: &rstyle_pdg::HirFnFacts) {
    assert_eq!(function["outcome"], "excluded", "{function}");
    assert!(
        facts
            .source_diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.construct == "macro_generated_function" })
    );
    let (unit, index) = data_unit(function);
    assert!(unit.pair(index).is_none());
    let captured = rstyle_pdg::data_receivers::verify_capture(facts).unwrap();
    assert!(!captured.methods.is_empty());
    assert!(rstyle_pdg::data_receivers::verify(facts).is_err());
    let mut corrupted = facts.clone();
    let mut capture = captured;
    capture.methods[0].receiver.local += 1;
    corrupted.build.frontend_options.insert(
        rstyle_pdg::data_receivers::KEY.into(),
        serde_json::to_value(capture).unwrap(),
    );
    assert!(rstyle_pdg::data_receivers::verify_capture(&corrupted).is_err());
}
