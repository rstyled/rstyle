use super::*;

fn workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        include_str!("../../../../eval/coverage-development/scoped-import-flag-owners.rs"),
    )
    .unwrap();
    std::fs::write(
        directory.path().join("src/macros.rs"),
        "macro_rules! harmless { () => { 42 }; }",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("src/exports.rs"),
        "pub struct Neighbor { pub value: u8 }",
    )
    .unwrap();
    directory
}

#[test]
fn closed_imports_connect_copy_constructor_getter_and_value_helper_snapshots() {
    let directory = workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["functions"].as_array().unwrap().len(), 6);
    for function in report["functions"].as_array().unwrap() {
        let expected = match function["facts"]["name"].as_str().unwrap() {
            "Flags::empty" | "Flags::is_empty" | "empty_flags" | "roundtrip" | "test_bits" => {
                "admitted"
            }
            "<Flags as std::clone::Clone>::clone" | "<Flags as core::clone::Clone>::clone" => {
                "excluded"
            }
            name => panic!("unexpected function {name}"),
        };
        assert_eq!(function["outcome"], expected, "{function}");
        if function["facts"]["name"] == "test_bits" {
            check_helper_boundary(function);
        }
    }
    check_aggregate_documents(&report);
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "Flags::is_empty")
        .unwrap();
    reject_policy_downgrade(function);

    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{report}");
    }
}

fn check_helper_boundary(function: &Value) {
    reject_helper(function, "effects");
    reject_helper(function, "source");
    for change in [
        "missing",
        "empty",
        "extra",
        "operation",
        "access",
        "parameter",
        "path",
        "type",
        "effect",
        "identity",
    ] {
        shared_evidence::reject_read_summary(function, change);
    }
    reject_snapshot_corruption(function);
}

fn reject_snapshot_corruption(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    let call = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| {
            node.annotations
                .contains_key("rstyle.data:owned-receiver-reads")
        })
        .unwrap();
    call.annotations.remove("rstyle.data:owned-receiver-reads");
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

#[test]
fn unresolved_import_effects_and_reexports_do_not_acquire_nominal_permission() {
    for (file, replacement) in [
        ("exports.rs", "pub type u8 = u64;"),
        ("exports.rs", "pub use core::marker::PhantomData;"),
        (
            "exports.rs",
            "macro_rules! make { () => { pub struct Other; }; } make!();",
        ),
        ("macros.rs", "macro_rules! u8 { () => { 42 }; }"),
        ("macros.rs", "pub const OTHER: usize = 0;"),
    ] {
        let directory = workspace();
        std::fs::write(directory.path().join("src").join(file), replacement).unwrap();
        let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
        assert!(output.status.success(), "{replacement}: {report}");
        let selected = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["facts"]["name"] == "Flags::is_empty")
            .unwrap();
        assert_eq!(selected["outcome"], "excluded", "{replacement}: {selected}");
    }
}

fn reject_policy_downgrade(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    pair.pdg.profile.version = "3.44.0".into();
    pair.cfg.profile.version = "3.44.0".into();
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

#[test]
fn external_globs_and_external_macro_imports_remain_unavailable() {
    for replacement in ["pub use core::cmp::*;", "#[macro_use] extern crate core;"] {
        let directory = workspace();
        let path = directory.path().join("src/lib.rs");
        let source = std::fs::read_to_string(&path).unwrap();
        let source = if replacement.starts_with("pub use") {
            source.replace("pub use crate::exports::*;", replacement)
        } else {
            source.replace("#[macro_use]\nmod macros;", replacement)
        };
        std::fs::write(path, source).unwrap();
        let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
        assert!(output.status.success(), "{report}");
        let selected = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["facts"]["name"] == "Flags::is_empty")
            .unwrap();
        assert_eq!(selected["outcome"], "excluded", "{replacement}: {selected}");
    }
}

#[test]
fn scoped_imports_do_not_invent_copy_or_erase_drop_behavior() {
    for destructor in ["", "impl Drop for Flags { fn drop(&mut self) {} }"] {
        let directory = workspace();
        let path = directory.path().join("src/lib.rs");
        let source = std::fs::read_to_string(&path)
            .unwrap()
            .replace("#[derive(Clone, Copy)]", "");
        std::fs::write(path, format!("{source}\n{destructor}")).unwrap();
        let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
        assert!(output.status.success(), "{report}");
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["facts"]["name"] == "test_bits")
            .unwrap();
        assert_eq!(function["outcome"], "excluded", "{function}");
    }
}

#[test]
fn scoped_nominal_permission_does_not_activate_historical_profiles() {
    let directory = workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{report}");
    assert!(
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["outcome"] == "excluded")
    );
}

#[test]
fn copy_snapshot_policy_is_required_even_without_scoped_imports() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        #[derive(Clone, Copy)] pub struct Flags(u64);
        impl Flags { pub fn is_empty(self) -> bool { self.0 == 0 } }
        pub fn test_bits(bits: u64) -> bool { Flags(bits).is_empty() }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "test_bits")
        .unwrap();
    assert_eq!(function["outcome"], "admitted", "{function}");
    reject_policy_downgrade(function);
}

#[test]
fn scoped_document_policy_covers_declaration_scopes_outside_the_function_module() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        mod model {
            #[macro_use] mod macros { macro_rules! harmless { () => { 42 }; } }
            #[derive(Clone, Copy)] pub struct Flags(pub u64);
        }
        pub fn inspect(value: model::Flags) -> u64 { value.0 }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "inspect")
        .unwrap();
    assert_eq!(function["outcome"], "admitted", "{function}");
    reject_policy_downgrade(function);
}
