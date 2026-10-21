use super::*;

const SOURCE: &str = include_str!("../../../../eval/coverage-development/look-flags.rs");
const ORACLE: &str = include_str!("../../../../eval/coverage-development/look-flags-oracle.rs");

#[test]
fn fieldless_flags_complete_value_construction_membership_and_grouped_helpers() {
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let library = Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output()
            .unwrap();
        assert!(library.status.success());
        if !Path::new(String::from_utf8_lossy(&library.stdout).trim()).is_dir() {
            continue;
        }
        for no_std in [false, true] {
            let directory = clone_workspace();
            std::fs::write(
                directory.path().join("src/lib.rs"),
                format!("{}{SOURCE}", if no_std { "#![no_std]\n" } else { "" }),
            )
            .unwrap();
            let (output, report) = clones(
                directory.path(),
                &["--extraction-profile", "hir-data-v3", "--target", target],
            );
            assert!(output.status.success(), "{}", report["capture_failures"]);
            let functions = report["functions"].as_array().unwrap();
            assert_eq!(functions.len(), 18);
            for function in functions {
                let name = function["facts"]["name"].as_str().unwrap();
                assert_eq!(
                    function["outcome"],
                    if name.starts_with('<') {
                        "excluded"
                    } else {
                        "admitted"
                    },
                    "{name}: {}",
                    function["reasons"]
                );
            }
            super::word_classifiers::verify_admissions(&report);
            assert_eq!(report["pair_evaluation_complete"], true);
            assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 91);
            reject_changed_value(&report);
        }
    }
}

fn reject_changed_value(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "LookSet::contains_anchor_lf")
        .unwrap();
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    let mut changed = rstyle_pdg::producer::Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    };
    let operation = changed
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("data.fieldless-value"))
        .unwrap();
    operation.attributes.literal = Some(rstyle_pdg::StringFact::known("999"));
    assert!(rstyle_pdg::data_documents::verify(&changed, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&changed, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&changed, &facts).is_err());
}

#[test]
fn fieldless_flags_follow_independent_masks_and_preserve_input_values() {
    runtime(&format!("{SOURCE}\n{ORACLE}"));
}

fn runtime(bytes: &str) {
    let directory = clone_workspace();
    let source = directory.path().join("oracle.rs");
    let binary = directory.path().join("oracle");
    std::fs::write(&source, bytes).unwrap();
    let compiled = Command::new("rustc")
        .args(["--test", "--edition=2021", "-O"])
        .arg(source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let executed = Command::new(binary).output().unwrap();
    assert!(
        executed.status.success(),
        "{}",
        String::from_utf8_lossy(&executed.stdout)
    );
}

#[test]
fn shared_copy_value_receivers_keep_original_reads_before_argument_rebinding() {
    let source = r#"
        #[derive(Clone, Copy)] pub struct Value { bits: u32 }
        impl Value {
            pub fn consume(self, extra: u32) -> u32 { self.bits ^ extra }
            pub fn value(self) -> Value { self }
        }
        pub fn copied(a: &Value) -> Value { a.value() }
        #[derive(Clone, Copy)] pub enum Flag { A = 1, B = 2 }
        impl Flag { pub fn code(self) -> u8 { self as u8 } }
        pub fn shared_flag(a: &Flag) -> u8 { a.code() }
        pub fn snapshot<'a>(mut a: &'a Value, b: &'a Value) -> u32 {
            a.consume({ a = b; 0 })
        }
        pub fn alias(a: &Value) -> u32 { a.consume(1) ^ a.consume(2) }
    "#;
    runtime(&format!(
        "{source}{}",
        r#"
        #[test] fn receiver_values_follow_source_evaluation_order() {
            let a = Value { bits: 7 }; let b = Value { bits: 9 };
            assert_eq!(snapshot(&a, &b), 7);
            assert_eq!(copied(&a).bits, 7);
            assert_eq!(shared_flag(&Flag::A), 1);
            assert_eq!(shared_flag(&Flag::B), 2);
            assert_eq!(alias(&a), 3);
        }
    "#
    ));
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    for function in report["functions"].as_array().unwrap() {
        if function["facts"]["name"].as_str().unwrap().starts_with('<') {
            continue;
        }
        assert_eq!(function["outcome"], "admitted", "{}", function["reasons"]);
        if matches!(
            function["facts"]["name"].as_str(),
            Some("copied" | "shared_flag")
        ) {
            check_whole_receiver_read(function);
        }
        if function["facts"]["name"] == "snapshot" {
            check_receiver_snapshot(function);
        }
    }
    super::word_classifiers::verify_admissions(&report);
    assert_eq!(report["pair_evaluation_complete"], true);
}

fn check_whole_receiver_read(function: &serde_json::Value) {
    let (unit, index) = data_unit(function);
    let pair = unit.pair(index).unwrap();
    assert_eq!(pair.pdg.memory_locations.len(), 1);
    assert_eq!(
        pair.pdg.memory_locations[0].extensions[rstyle_pdg::data_documents::LOCATION_KEY]["parameter"],
        0
    );
}

fn check_receiver_snapshot(function: &serde_json::Value) {
    let (unit, index) = data_unit(function);
    let pair = unit.pair(index).unwrap();
    assert!(!pair.pdg.memory_locations.is_empty());
    for location in &pair.pdg.memory_locations {
        assert_eq!(
            location.extensions[rstyle_pdg::data_documents::LOCATION_KEY]["parameter"],
            0
        );
    }
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let mut changed = rstyle_pdg::producer::Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    };
    let call = changed
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.category == rstyle_pdg::NodeCategory::Call)
        .unwrap();
    call.attributes.reads.clear();
    assert!(rstyle_pdg::data_documents::verify(&changed, &facts).is_err());
    shared_evidence::reject_edit(function, |bundle, index| {
        bundle["functions"][index]["build"]["frontend_options"]["data_method_receivers"]["methods"]
            [0]["adjustments"] = serde_json::json!([]);
    });
}

#[test]
fn fieldless_enum_support_retains_signed_values_and_closed_declaration_boundaries() {
    let directory = clone_workspace();
    let source = r#"
        #[derive(Clone, Copy)] pub enum Signed { Negative = -7, High = 1 << 5, Next }
        pub fn signed(value: Signed) -> i8 { value as i8 }
        pub fn unsigned(value: Signed) -> u8 { value as u8 }
        pub fn negative() -> Signed { Signed::Negative }
        #[repr(u8)] #[derive(Clone, Copy)] pub enum Repr { A, B }
        pub fn custom_repr(value: Repr) -> u8 { value as u8 }
        #[derive(Clone, Copy)] pub enum Conditional { #[cfg(any())] A, B }
        pub fn conditional(value: Conditional) -> u8 { value as u8 }
        const MASK: isize = 1;
        #[derive(Clone, Copy)] pub enum Named { A = MASK, B = 4 }
        pub fn named(value: Named) -> u8 { value as u8 }
        pub enum Owned { A, B }
        pub fn owned(value: Owned) -> u8 { value as u8 }
        #[derive(Clone)] pub struct Manual { bits: u32 }
        impl Copy for Manual {}
        impl Manual { pub fn consume(self) -> u32 { self.bits } }
        pub fn custom_copy(value: &Manual) -> u32 { value.consume() }
    "#;
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    for function in report["functions"].as_array().unwrap() {
        let name = function["facts"]["name"].as_str().unwrap();
        let expected = matches!(name, "signed" | "unsigned" | "negative");
        assert_eq!(
            function["outcome"],
            if expected { "admitted" } else { "excluded" },
            "{name}: {}",
            function["reasons"]
        );
    }
    super::word_classifiers::verify_admissions(&report);
}
