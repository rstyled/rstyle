//! Concrete Default composition retains original bodies and selected trait authority.
use super::*;

macro_rules! fixture {
    ($($source:tt)*) => {
        mod oracle { $($source)* }
        const SOURCE: &str = stringify!($($source)*);
    };
}

fixture! {
    #[derive(Copy, Clone)]
    pub struct State { a: u16, b: u16 }
    impl State {
        pub fn new() -> Self { Self::default() }
        pub fn checksum(&self) -> u32 { ((self.b as u32) << 16) | self.a as u32 }
    }
    impl Default for State {
        fn default() -> Self { State { a: 1, b: 0 } }
    }
    pub fn initialized() -> u32 { State::new().checksum() }
    pub fn same() -> u32 { State::new().checksum() }
}

#[test]
fn concrete_default_composes_original_initializer_and_getter_in_both_preludes() {
    assert_eq!(oracle::State::new().checksum(), 1);
    assert_eq!(oracle::initialized(), 1);
    assert_eq!(oracle::same(), 1);
    for no_std in [false, true] {
        check_context(no_std);
    }
}

fn check_context(no_std: bool) {
    let directory = clone_workspace();
    let prefix = if no_std {
        "#![no_std]\nextern crate core as std;\n"
    } else {
        ""
    };
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{prefix}{SOURCE}"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let functions = report["functions"].as_array().unwrap();
    check_roster(functions);
    check_aggregate_documents(&report);
    check_selection(functions);
    reject_historical_policy(functions);
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_evaluation_complete",
        "pair_enumeration_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    assert!(
        report["matching"]["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|pair| pair["evaluation_status"] == "evaluated")
    );
}

fn check_roster(functions: &[Value]) {
    assert_eq!(functions.len(), 6);
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        let positive = name.ends_with("State::new")
            || name.ends_with("State::checksum")
            || name.ends_with("::default")
            || matches!(name, "initialized" | "same");
        assert!(
            positive || name.ends_with("::clone"),
            "Unexpected captured neighbor: {name}"
        );
        assert_eq!(
            function["outcome"],
            if positive { "admitted" } else { "excluded" },
            "{name}: {}",
            function["reasons"]
        );
    }
}

fn check_selection(functions: &[Value]) {
    let function = functions
        .iter()
        .find(|function| {
            function["facts"]["name"]
                .as_str()
                .unwrap()
                .ends_with("State::new")
        })
        .unwrap();
    let call = &function["facts"]["body"]["children"][0];
    let resolution: Value =
        serde_json::from_str(call["res"]["description"].as_str().unwrap()).unwrap();
    let selected = &resolution["default_selection"];
    assert_eq!(
        selected["identity"],
        "rstyle.rust.concrete-default-call-observation/1"
    );
    assert_eq!(selected["admission_authority"], false);
    assert_eq!(selected["requested"]["crate"], "core");
    assert_eq!(selected["selected"]["local"], true);
    assert_eq!(
        selected["selected_signature"]["inputs"],
        serde_json::json!([])
    );
    for field in [
        "requested",
        "selected",
        "implementation",
        "requested_signature",
    ] {
        let mut facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let mut changed = resolution.clone();
        changed["default_selection"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        facts.body.children[0].res.as_mut().unwrap().description = changed.to_string();
        shared_evidence::reject_facts(function, &facts);
    }
    reject_changed_selection(function, &resolution);
    reject_instance_inventory(function);
}

fn reject_changed_selection(function: &Value, resolution: &Value) {
    for attack in [
        "core_crate",
        "trait_identity",
        "method_identity",
        "implementation",
        "selected",
        "nominal",
    ] {
        let mut changed = resolution.clone();
        let selection = &mut changed["default_selection"];
        match attack {
            "core_crate" => selection["requested"]["stable_crate_id"] = "forged-core".into(),
            "trait_identity" => {
                selection["trait_definition"]["definition_identity"] = "forged-trait".into();
            }
            "method_identity" => {
                selection["requested"]["definition_identity"] = "forged-method".into();
            }
            "implementation" => {
                selection["implementation"]["definition_identity"] = "forged-impl".into();
            }
            "selected" => {
                selection["selected"]["definition_identity"] = "forged-selected".into();
            }
            "nominal" => {
                selection["requested_arguments"][0]["identity"]["definition"]["path"] =
                    "clone_fixture::OtherState".into();
            }
            _ => unreachable!(),
        }
        let mut facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        facts.body.children[0].res.as_mut().unwrap().description = changed.to_string();
        shared_evidence::reject_facts(function, &facts);
    }
}

fn reject_instance_inventory(function: &Value) {
    const KEY: &str = "rstyle.rust.call-instances";
    for attack in [
        "missing",
        "empty",
        "duplicate",
        "requested",
        "selected",
        "site",
        "incomplete",
    ] {
        let mut facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let mut inventory = facts.build.frontend_options.get(KEY).unwrap().clone();
        match attack {
            "missing" => {
                facts.build.frontend_options.remove(KEY);
            }
            "empty" => inventory["calls"] = serde_json::json!([]),
            "duplicate" => {
                let call = inventory["calls"][0].clone();
                inventory["calls"].as_array_mut().unwrap().push(call);
            }
            "requested" => {
                inventory["calls"][0]["requested"]["definition_identity"] =
                    "forged-inherent-default".into()
            }
            "selected" => {
                inventory["calls"][0]["selected"]["definition_identity"] = "forged-selected".into()
            }
            "site" => inventory["calls"][0]["span"]["start"] = 0.into(),
            "incomplete" => inventory["complete"] = false.into(),
            _ => unreachable!(),
        }
        if attack != "missing" {
            facts.build.frontend_options.insert(KEY.into(), inventory);
        }
        shared_evidence::reject_facts(function, &facts);
    }
}

#[test]
fn concrete_default_does_not_certify_shadow_traits_generics_or_drop_owners() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        mod shadow {
            pub trait Default { fn default() -> Self; }
            #[derive(Copy, Clone)] pub struct State { value: u16 }
            impl Default for State { fn default() -> Self { State { value: 7 } } }
            pub fn shadowed() -> State { State::default() }
        }
        pub struct Generic<T> { value: T }
        impl<T: Default> Default for Generic<T> { fn default() -> Self { Self { value: T::default() } } }
        pub fn generic() -> Generic<u16> { Generic::default() }
        pub struct Owned { value: u16 }
        impl Drop for Owned { fn drop(&mut self) {} }
        impl Default for Owned { fn default() -> Self { Owned { value: 7 } } }
        pub fn owned() -> Owned { Owned::default() }
        #[derive(Copy, Clone)] pub struct Plain { value: u16 }
        impl Plain { pub fn default() -> Self { Plain { value: 9 } } }
        pub fn inherent() -> Plain { Plain::default() }
    "#).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let functions = report["functions"].as_array().unwrap();
    for name in ["shadow::shadowed", "generic", "owned"] {
        let function = functions
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        assert_eq!(function["outcome"], "excluded", "{name}");
    }
    let inherent = functions
        .iter()
        .find(|function| function["facts"]["name"] == "inherent")
        .unwrap();
    assert!(
        !inherent["facts"]["body"]
            .to_string()
            .contains("default_selection")
    );
}

#[test]
fn concrete_default_requires_the_implicit_prelude_source_contract() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        #![no_implicit_prelude]
        extern crate core;
        pub struct State { value: u16 }
        impl core::default::Default for State { fn default() -> Self { State { value: 1 } } }
        pub fn new() -> State { <State as core::default::Default>::default() }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(
            function["outcome"], "excluded",
            "{}",
            function["facts"]["name"]
        );
    }
}

fn reject_historical_policy(functions: &[Value]) {
    for function in functions.iter().filter(|function| {
        function["facts"]["name"]
            .as_str()
            .is_some_and(|name| name.ends_with("State::new") || name.ends_with("::default"))
    }) {
        let (store, index) = shared_evidence::load(function);
        let pair = store.pair(index).unwrap();
        let historical = shared_evidence::with_profile_version(pair, "3.93.0");
        assert!(
            rstyle_pdg::data_documents::verify(
                &historical,
                &store.analysis().function(index).unwrap()
            )
            .is_err()
        );
        let sealed = historical.serialize().unwrap();
        let analysis = store.analysis();
        let facts = analysis.function(index).unwrap();
        assert!(
            rstyle_pdg::verification_policy::strict(|| {
                rstyle_pdg::producer::verify_serialized(&sealed, &facts)
            })
            .is_err()
        );
    }
}
