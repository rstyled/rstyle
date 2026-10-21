//! Mixed-unit endpoints, supported chains and the owned Copy receiver bridge.
use super::*;

#[test]
fn selected_copy_family_composes_in_complete_mixed_cargo_units() {
    let directory = workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let positive = [
        "data::composed",
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
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 45);
    assert_eq!(report["pair_evaluation_complete"], true);
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

#[test]
fn owned_copy_receivers_preserve_snapshot_reads_and_mixed_shared_arguments() {
    let directory = workspace();
    let source = format!(
        "{}\n{}",
        include_str!("../../../../eval/coverage-development/position-span.rs"),
        r#"
        impl Span {
            pub fn compare(&self, other: &Span) -> bool { self.start.offset == other.end.offset }
            pub fn escape(&self) -> &Position { &self.start }
        }
        pub fn owned(span: Span) -> bool { span.is_empty() }
        pub fn result_receiver(x: usize, y: usize) -> bool {
            Span::new(Position::new(x, 1, 2), Position::new(y, 1, 3)).is_empty()
        }
        pub fn repeated(mut span: Span, other: Span) -> bool {
            let first = span.is_empty(); span = other; first == span.is_empty()
        }
        pub fn mixed(span: Span, other: &Span) -> bool { span.compare(other) }
        pub fn escaped(span: Span) -> usize { span.escape().offset }
        "#
    );
    std::fs::write(directory.path().join("src/data.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    for name in ["owned", "result_receiver", "repeated", "mixed"] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == format!("data::{name}"))
            .unwrap();
        assert_eq!(function["outcome"], "admitted", "{function}");
        check_snapshot_reads(function, name == "mixed");
        if name == "owned" {
            check_owned_adjustments(function);
        }
    }
    let escaped = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "data::escaped")
        .unwrap();
    assert_eq!(escaped["outcome"], "excluded", "{escaped}");
    check_aggregate_documents(&report);
}

fn check_snapshot_reads(function: &Value, mixed: bool) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = shared_evidence::pair(&unit, function);
    let calls: Vec<_> = original
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.annotations
                .contains_key("rstyle.data:owned-receiver-reads")
        })
        .collect();
    assert!(!calls.is_empty());
    for call in calls {
        let reads: Vec<rstyle_pdg::helper_reads::Read> =
            serde_json::from_value(call.annotations["rstyle.data:owned-receiver-reads"].clone())
                .unwrap();
        check_snapshot_paths(&reads, mixed);
        assert_eq!(reads.len(), if mixed { 1 } else { 2 });
        assert_eq!(call.attributes.reads.len(), if mixed { 1 } else { 0 });
        for mutation in ["missing", "path", "duplicate", "type"] {
            let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
            corrupt_snapshot(&mut pair, &call.id, mutation);
            assert!(
                rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
                "{mutation}"
            );
            assert!(
                rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
                "{mutation}"
            );
        }
    }
}

fn corrupt_snapshot(pair: &mut rstyle_pdg::producer::Pair, id: &str, mutation: &str) {
    let changed = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.id == id)
        .unwrap();
    let annotation = changed
        .annotations
        .get_mut("rstyle.data:owned-receiver-reads")
        .unwrap();
    match mutation {
        "missing" => {
            changed
                .annotations
                .remove("rstyle.data:owned-receiver-reads");
        }
        "path" => annotation[0]["path"] = serde_json::json!([]),
        "duplicate" => {
            let extra = annotation[0].clone();
            annotation.as_array_mut().unwrap().push(extra);
        }
        "type" => annotation[0]["value"] = serde_json::json!({"kind":"scalar", "value":"bool"}),
        _ => unreachable!(),
    }
}

fn check_owned_adjustments(function: &Value) {
    use rstyle_pdg::data_receivers::{self, Kind};
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = data_receivers::verify(&facts).unwrap();
    for mutation in ["missing", "extra", "mutable", "target"] {
        let mut capture = original.clone();
        let record = &mut capture.methods[0];
        match mutation {
            "missing" => record.adjustments.clear(),
            "extra" => record.adjustments.push(record.adjustments[0].clone()),
            "mutable" => record.adjustments[0].kind = Kind::MutableBorrow,
            "target" => record.adjustments[0].target = record.receiver_type.clone(),
            _ => unreachable!(),
        }
        let mut changed = facts.clone();
        changed.build.frontend_options.insert(
            data_receivers::KEY.into(),
            serde_json::to_value(capture).unwrap(),
        );
        assert!(
            rstyle_pdg::producer::produce(&changed).is_err(),
            "{mutation}"
        );
        shared_evidence::reject_facts(function, &changed);
    }
}

fn check_snapshot_paths(reads: &[rstyle_pdg::helper_reads::Read], mixed: bool) {
    use rstyle_pdg::{data_types::TypeToken, field_locations::Projection};
    let mut paths = Vec::new();
    for read in reads {
        assert_eq!(read.parameter, 0);
        assert_eq!(read.value, TypeToken::Scalar("usize".into()));
        assert_eq!(read.path.first(), Some(&Projection::SharedReferent));
        assert_eq!(read.path.len(), 3);
        let names: Vec<_> = read
            .path
            .iter()
            .filter_map(|part| match part {
                Projection::StructField { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        paths.push(names.join("."));
    }
    assert_eq!(
        paths,
        if mixed {
            vec!["start.offset"]
        } else {
            vec!["start.offset", "end.offset"]
        }
    );
}

#[test]
fn copy_value_self_updates_compose_with_results_getters_and_ufcs() {
    let directory = update_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    check_value_self_outcomes(&report);
    for (name, slot) in [("data::Span::with_start", 0), ("data::Span::with_end", 1)] {
        check_value_update(read_call_function(&report, name), slot);
        check_value_owner(read_call_function(&report, name));
    }
    check_value_adjustment(read_call_function(&report, "data::updated_both"));
    assert_eq!(
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|function| function["outcome"] == "admitted")
            .count(),
        18
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 153);
    assert_eq!(report["pair_evaluation_complete"], true);
    check_aggregate_documents(&report);
}

fn check_value_update(function: &Value, slot: usize) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    let construct = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("data.struct"))
        .unwrap();
    assert_eq!(
        construct.attributes.literal.as_ref().unwrap().known_value(),
        Some(format!("[{slot},null]").as_str())
    );
    construct.attributes.literal = Some(rstyle_pdg::StringFact::known(&format!(
        "[{},null]",
        1 - slot
    )));
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
    let mut changed = facts.clone();
    let mut origins = changed
        .build
        .frontend_options
        .data::<rstyle_pdg::declaration_origins::Origins>()
        .unwrap()
        .as_ref()
        .clone();
    for origin in &mut origins.declarations {
        origin.builtin_copy = None;
    }
    changed.build.frontend_options.insert(
        rstyle_pdg::declaration_origins::KEY.into(),
        serde_json::to_value(origins).unwrap(),
    );
    assert!(rstyle_pdg::producer::produce(&changed).is_err());
    shared_evidence::reject_facts(function, &changed);
}

fn check_value_adjustment(function: &Value) {
    use rstyle_pdg::{
        concrete_types::{Observation, TypeTree},
        data_receivers::{self, Adjustment, Kind},
    };
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let mut capture = data_receivers::verify(&facts).unwrap();
    assert_eq!(capture.methods.len(), 2);
    assert!(
        capture
            .methods
            .iter()
            .all(|record| record.adjustments.is_empty())
    );
    let record = &mut capture.methods[0];
    let Observation::Captured { ty } = &record.receiver_type else {
        panic!("missing owned type")
    };
    let shared = Observation::Captured {
        ty: TypeTree::Shared(Box::new(ty.clone())),
    };
    record.adjusted_type = shared.clone();
    record.adjustments.push(Adjustment {
        kind: Kind::SharedBorrow,
        target: shared,
    });
    let mut changed = facts.clone();
    changed.build.frontend_options.insert(
        data_receivers::KEY.into(),
        serde_json::to_value(capture).unwrap(),
    );
    assert!(rstyle_pdg::producer::produce(&changed).is_err());
    shared_evidence::reject_facts(function, &changed);
}

fn update_workspace() -> TempDir {
    let directory = workspace();
    std::fs::write(
        directory.path().join("src/data.rs"),
        format!(
            "{}\n{}",
            include_str!("../../../../eval/coverage-development/position-span.rs"),
            include_str!("../../../../eval/coverage-development/span-updates.rs")
        ),
    )
    .unwrap();
    directory
}

fn check_value_self_outcomes(report: &Value) {
    let positive = [
        "data::Span::with_start",
        "data::Span::with_end",
        "data::updated_both",
        "data::updated_empty",
        "data::result_update",
        "data::ufcs_update",
        "data::retained",
        "data::shared_to_owned",
    ];
    let negative = ["data::Span::mutable_self", "data::Span::typed_self"];
    for name in positive.into_iter().chain(negative) {
        let function = read_call_function(report, name);
        assert_eq!(
            function["outcome"],
            if positive.contains(&name) {
                "admitted"
            } else {
                "excluded"
            },
            "{function}"
        );
    }
}

#[test]
fn copy_value_self_updates_preserve_installed_ci_contexts() {
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
        let directory = update_workspace();
        let (output, report) = clones(
            directory.path(),
            &["--target", target, "--extraction-profile", "hir-data-v3"],
        );
        assert!(output.status.success(), "{target}: {report}");
        check_value_self_outcomes(&report);
        check_aggregate_documents(&report);
        assert_eq!(report["pair_evaluation_complete"], true);
    }
}

#[test]
fn value_self_calls_bind_observed_signature_to_the_actual_callee() {
    use rstyle_pdg::{
        concrete_types::{Observation, TypeTree},
        data_receivers::{self, Adjustment, Kind},
    };
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        #[derive(Clone, Copy)] pub struct Data { pub n: u8 }
        impl Data { pub fn get(self) -> u8 { 7 } }
        pub fn caller(value: Data) -> u8 { value.get() }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let function = read_call_function(&report, "caller");
    assert_eq!(function["outcome"], "admitted", "{function}");
    let (unit, index) = data_unit(function);
    let mut functions = unit.functions().to_vec();
    let changed = &mut functions[index];
    let mut capture = data_receivers::verify(changed).unwrap();
    let record = &mut capture.methods[0];
    let Observation::Captured { ty } = &record.receiver_type else {
        panic!("missing type")
    };
    let borrowed = Observation::Captured {
        ty: TypeTree::Shared(Box::new(ty.clone())),
    };
    record.adjusted_type = borrowed.clone();
    record.signature_inputs[0] = borrowed.clone();
    record.adjustments.push(Adjustment {
        kind: Kind::SharedBorrow,
        target: borrowed,
    });
    changed.build.frontend_options.insert(
        data_receivers::KEY.into(),
        serde_json::to_value(capture).unwrap(),
    );
    // The observation is internally well-formed; binding it to the actual
    // by-value callee must still fail rather than trusting its adjusted inputs.
    data_receivers::verify(changed).unwrap();
    let replay = rstyle_pdg::unit_store::UnitStore::capture(functions).unwrap();
    assert_eq!(replay.functions().len(), unit.functions().len());
    assert!(replay.pair(index).is_none());
}

fn check_value_owner(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let origins = facts
        .build
        .frontend_options
        .data::<rstyle_pdg::declaration_origins::Origins>()
        .unwrap();
    let other = origins
        .declarations
        .iter()
        .find(|origin| origin.nominal.definition_path.ends_with("::Position"))
        .unwrap()
        .nominal
        .clone();
    let mut changed = facts.clone();
    substitute_value_owner(&mut changed, other);
    assert!(rstyle_pdg::producer::produce(&changed).is_err());
    shared_evidence::reject_facts(function, &changed);
}

fn substitute_value_owner(
    facts: &mut rstyle_pdg::HirFnFacts,
    other: rstyle_pdg::concrete_types::Nominal,
) {
    use rstyle_pdg::inherent_calls::{self, Definition, UnitDefinitions};
    let mut record: Definition = serde_json::from_value(
        facts
            .build
            .frontend_options
            .get(inherent_calls::KEY)
            .unwrap()
            .clone(),
    )
    .unwrap();
    let mut definitions: UnitDefinitions = serde_json::from_value(
        facts
            .build
            .frontend_options
            .get(inherent_calls::UNIT_KEY)
            .unwrap()
            .clone(),
    )
    .unwrap();
    record.owner_nominal = other.clone();
    for item in &mut definitions.definitions {
        if item.implementation_identity == record.implementation_identity {
            item.owner_nominal = other.clone();
        }
    }
    facts.build.frontend_options.insert(
        inherent_calls::KEY.into(),
        serde_json::to_value(record).unwrap(),
    );
    facts.build.frontend_options.insert(
        inherent_calls::UNIT_KEY.into(),
        serde_json::to_value(definitions).unwrap(),
    );
}
