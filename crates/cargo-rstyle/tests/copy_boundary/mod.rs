//! Mixed-unit endpoints, supported chains and the excluded local-autoref bridge.
use super::*;

#[test]
fn selected_copy_family_composes_in_complete_mixed_cargo_units() {
    let directory = workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let positive = [
        "data::Position::new",
        "data::Span::new",
        "data::Span::is_empty",
        "data::Span::is_one_line",
        "data::Span::splat",
        "data::constructed_fields",
        "data::shared_getter",
        "data::same_empty",
        "data::different_empty",
    ];
    for name in positive {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        assert_eq!(function["outcome"], "admitted", "{function}");
    }
    check_kept_exclusions(&report);
    check_getter_paths(&report, "data::Span::is_empty", "offset");
    check_getter_paths(&report, "data::Span::is_one_line", "line");
    check_getter_paths(&report, "data::different_empty", "column");
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["data::Span::is_empty", "data::same_empty"]);
    check_field_matching_evaluates_completely(&report);
}

fn workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "#![no_std]\npub mod data;\npub mod unrelated;\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("src/data.rs"),
        include_str!("../../../../eval/coverage-development/position-span.rs"),
    )
    .unwrap();
    std::fs::write(
        directory.path().join("src/unrelated.rs"),
        r#"
        pub struct Dropped;
        impl Drop for Dropped { fn drop(&mut self) {} }
        pub struct Generic<T> { pub value: T }
        pub enum Noise { A, B }
        macro_rules! noise { () => { pub fn generated() -> usize { 7 } } }
        noise!();
        use core::cmp::{self, Ordering};
        use core::fmt::{self, Debug};
        pub fn plain(value: &Dropped) -> usize { 1 }
    "#,
    )
    .unwrap();
    directory
}

#[test]
fn selected_copy_boundary_keeps_missing_proofs_and_lexical_unknowns_closed() {
    for change in ["copy", "macro", "primitive_alias", "glob"] {
        let directory = workspace();
        let original = include_str!("../../../../eval/coverage-development/position-span.rs");
        let source = match change {
            "copy" => original
                .replace("Clone, Copy, Eq, PartialEq", "Clone, Eq, PartialEq")
                .replace(
                    "Span::new(position, position)",
                    "Span::new(position.clone(), position)",
                ),
            "macro" => format!(
                "{original}\nmacro_rules! hidden {{ () => {{ pub fn hidden() {{}} }} }} hidden!();"
            ),
            "primitive_alias" => format!("{original}\ntype usize = core::primitive::usize;"),
            "glob" => format!("{original}\nuse core::fmt::*;"),
            _ => unreachable!(),
        };
        std::fs::write(directory.path().join("src/data.rs"), source).unwrap();
        let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
        assert!(output.status.success(), "{change}: {report}");
        for name in [
            "data::Position::new",
            "data::Span::new",
            "data::Span::is_empty",
            "data::Span::is_one_line",
        ] {
            let function = report["functions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|function| function["facts"]["name"] == name)
                .unwrap();
            assert_eq!(function["outcome"], "excluded", "{change}: {function}");
        }
    }
}

fn check_getter_paths(report: &Value, name: &str, field: &str) {
    use rstyle_pdg::field_locations::{FieldObservations, Location, Projection};
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == name)
        .unwrap();
    let observations: FieldObservations = serde_json::from_value(
        function["facts"]["build"]["frontend_options"][rstyle_pdg::field_locations::KEY].clone(),
    )
    .unwrap();
    let paths = observations
        .operations
        .iter()
        .filter_map(|operation| {
            let Location::Captured { path, .. } = &operation.location else {
                return None;
            };
            let names: Vec<_> = path
                .iter()
                .filter_map(|projection| match projection {
                    Projection::StructField { name, .. } => Some(name.as_str()),
                    _ => None,
                })
                .collect();
            (names.len() == 2).then_some(names)
        })
        .collect::<Vec<_>>();
    assert_eq!(paths.len(), 2);
    assert!(paths.contains(&vec!["start", field]), "{paths:?}");
    assert!(paths.contains(&vec!["end", field]), "{paths:?}");
}

#[test]
fn builtin_copy_does_not_authorize_an_additional_custom_derive() {
    let directory = super::nominal_copy::custom_derive_workspace();
    let macro_path = directory.path().join("custom/src/lib.rs");
    let original = std::fs::read_to_string(&macro_path).unwrap();
    std::fs::write(macro_path, format!("{original}\n#[proc_macro_derive(Noise)] pub fn noise(_: TokenStream) -> TokenStream {{ TokenStream::new() }}")).unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), "#[derive(Clone, Copy, custom::Noise)] pub struct Data { pub value: usize }\npub fn getter(value: &Data) -> usize { value.value }\n").unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "getter")
        .unwrap();
    assert!(function["facts"]["build"]["frontend_options"]["concrete_declaration_origins"]["declarations"][0]["builtin_copy"].is_object());
    assert_eq!(function["outcome"], "excluded");
    assert!(
        function["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason
                .as_str()
                .unwrap()
                .contains("unresolved lexical expansion"))
    );
}

fn check_kept_exclusions(report: &Value) {
    let composed = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "data::composed")
        .unwrap();
    assert_eq!(composed["outcome"], "excluded");
    assert!(
        composed["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason
                .as_str()
                .unwrap()
                .contains("helper identity, signature"))
    );
    let generated = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "unrelated::generated")
        .unwrap();
    assert_eq!(generated["outcome"], "excluded");
}

fn check_field_matching_evaluates_completely(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    let keys = ["data::Span::is_empty", "data::different_empty"].map(|name| {
        functions
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap()["key"]
            .clone()
    });
    let pair = report["matching"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| {
            (pair["first"] == keys[0] && pair["second"] == keys[1])
                || (pair["first"] == keys[1] && pair["second"] == keys[0])
        })
        .unwrap();
    assert_eq!(pair["evaluation_status"], "evaluated");
    // Similarity is not field-path equivalence. Exact source paths are checked
    // independently above; the detector's score is a separate quality claim.
}
