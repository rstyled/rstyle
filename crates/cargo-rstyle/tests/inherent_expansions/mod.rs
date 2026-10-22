//! Expanded methods retain function facts without claiming original ownership.
use super::*;
use serde_json::json;

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
    let mut capture = captured.into_owned();
    capture.methods[0].receiver.local += 1;
    corrupted.build.frontend_options.insert(
        rstyle_pdg::data_receivers::KEY.into(),
        serde_json::to_value(capture).unwrap(),
    );
    assert!(rstyle_pdg::data_receivers::verify_capture(&corrupted).is_err());
}

#[test]
fn inherent_inventory_sharing_preserves_strict_current_and_historical_imports() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        #[derive(Clone, Copy)] pub struct Data { value: u8 }
        impl Data { pub fn byte(&self) -> u8 { self.value } }
        pub fn forward(data: &Data) -> u8 { data.byte() }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| {
            function["facts"]["name"]
                .as_str()
                .unwrap()
                .ends_with("::byte")
        })
        .unwrap();
    assert_eq!(function["outcome"], "admitted");
    let path = function["unit_evidence"]["bundle"].as_str().unwrap();
    let wire: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(wire["schema"], rstyle_pdg::unit_store::BUNDLE_SCHEMA);
    assert_eq!(wire["inherent_units"].as_array().unwrap().len(), 1);
    check_strict_inherent_wire(&wire).unwrap();
    let legacy = historical_inherent_wire(wire.clone());
    check_strict_inherent_wire(&legacy).unwrap();
    check_inherent_inventory_tampering(wire);
    let mut changed = legacy;
    for function in changed["functions"].as_array_mut().unwrap() {
        function["build"]["frontend_options"]["inherent_unit_definitions"]["definitions"] =
            json!([]);
    }
    assert!(check_strict_inherent_wire(&changed).is_err());
}

fn check_strict_inherent_wire(wire: &Value) -> Result<rstyle_pdg::unit_store::UnitStore, String> {
    rstyle_pdg::unit_store::UnitStore::from_bundle(&serde_json::to_vec(wire).unwrap())
}

fn historical_inherent_wire(mut wire: Value) -> Value {
    let inventories = wire
        .as_object_mut()
        .unwrap()
        .remove("inherent_units")
        .unwrap();
    wire["schema"] = json!("rstyle.shared-unit/4");
    for function in wire["functions"].as_array_mut().unwrap() {
        let inventory = &mut function["build"]["frontend_options"]["inherent_unit_definitions"];
        let index = inventory["value"].as_u64().unwrap() as usize;
        *inventory = inventories[index].clone();
    }
    wire
}

fn check_inherent_inventory_tampering(wire: Value) {
    let mut missing = wire.clone();
    missing.as_object_mut().unwrap().remove("inherent_units");
    assert!(check_strict_inherent_wire(&missing).is_err());
    let mut null = wire.clone();
    null["inherent_units"] = Value::Null;
    assert!(check_strict_inherent_wire(&null).is_err());
    let mut replaced = wire.clone();
    replaced["inherent_units"][0]["definitions"] = json!([]);
    assert!(check_strict_inherent_wire(&replaced).is_err());
    let mut dangling = wire.clone();
    dangling["functions"][0]["build"]["frontend_options"]["inherent_unit_definitions"]["value"] =
        json!(u32::MAX);
    assert!(check_strict_inherent_wire(&dangling).is_err());
    let mut absent = wire;
    for function in absent["functions"].as_array_mut().unwrap() {
        function["build"]["frontend_options"]
            .as_object_mut()
            .unwrap()
            .remove("inherent_unit_definitions");
    }
    // Remove the now-unreferenced table as well: rejection must still exercise
    // the required source association rather than only unused table rejection.
    absent["inherent_units"] = json!([]);
    assert!(check_strict_inherent_wire(&absent).is_err());
}
