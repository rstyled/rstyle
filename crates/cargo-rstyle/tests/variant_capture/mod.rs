use super::*;
mod accelerator_packing;
mod bitset_cardinality;
mod borrowed_accel;
mod boundaries;
mod byte_comparisons;
mod classifiers;
mod corruption;
mod end_of_input;
mod iterator_position;
mod iterator_position_corruption;
mod word_classifiers;

fn descendants<'a>(node: &'a rstyle_pdg::HirNode, output: &mut Vec<&'a rstyle_pdg::HirNode>) {
    output.push(node);
    for child in &node.children {
        descendants(child, output);
    }
}

#[test]
fn scalar_enum_family_completes_capture_and_preserves_historical_patterns() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        include_str!("../../../../eval/coverage-development/tagged-units.rs"),
    )
    .unwrap();
    for profile in ["hir-data-v3", "hir-data-v2"] {
        let (output, report) = clones(directory.path(), &["--extraction-profile", profile]);
        assert!(output.status.success(), "{report}");
        let functions = report["functions"].as_array().unwrap();
        assert_eq!(functions.len(), 11);
        check_family_outcomes(&report, profile);
        for name in ["Unit::byte", "Unit::end"] {
            let function = functions
                .iter()
                .find(|f| f["facts"]["name"] == name)
                .unwrap();
            let facts: rstyle_pdg::HirFnFacts =
                serde_json::from_value(function["facts"].clone()).unwrap();
            check_patterns(&facts, profile);
            assert_eq!(
                rstyle_pdg::producer::produce(&facts).is_ok(),
                profile == "hir-data-v3"
            );
        }
    }
}

fn check_family_outcomes(report: &Value, profile: &str) {
    let functions = report["functions"].as_array().unwrap();
    for function in functions {
        let positive = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "Unit::new_byte" | "Unit::new_end" | "Unit::byte" | "Unit::end" | "roundtrip" | "saved"
        );
        assert_eq!(
            function["outcome"],
            if profile == "hir-data-v3" && positive {
                "admitted"
            } else {
                "excluded"
            },
            "{function}"
        );
    }
    if profile == "hir-data-v3" {
        check_aggregate_documents(report);
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
}

fn check_patterns(facts: &rstyle_pdg::HirFnFacts, profile: &str) {
    let mut nodes = Vec::new();
    descendants(&facts.body, &mut nodes);
    let patterns: Vec<_> = nodes
        .iter()
        .filter(|n| n.kind == "pat_local_variant")
        .collect();
    if profile != "hir-data-v3" {
        assert!(patterns.is_empty());
        assert_eq!(
            nodes.iter().filter(|n| n.kind == "pat_unsupported").count(),
            2
        );
        return;
    }
    assert_eq!(patterns.len(), 2);
    for (pattern, suffix) in patterns.iter().zip(["Kind::Byte", "Kind::End"]) {
        let resolution = pattern.res.as_ref().unwrap();
        assert_eq!(resolution.kind, "resolved_local_variant");
        let identity: rstyle_pdg::data_library::Callee =
            serde_json::from_str(&resolution.description).unwrap();
        assert!(identity.path.ends_with(suffix), "{identity:?}");
        assert_eq!(identity.crate_name, "clone_fixture");
        assert!(!identity.definition_hash.is_empty());
        assert_eq!(pattern.children.len(), 1);
    }
    let bindings: Vec<_> = patterns
        .iter()
        .flat_map(|n| &n.children)
        .filter(|n| n.kind == "pat_binding")
        .collect();
    assert_eq!(bindings.len(), 1);
    let binding = bindings[0].binding.as_ref().unwrap();
    assert!(facts.bindings.iter().any(|fact| &fact.symbol == binding));
    assert!(nodes.iter().any(|n| n.kind == "expr_path"
        && n.res.as_ref().and_then(|res| res.target.as_ref()) == Some(&bindings[0].hir)));
}

#[test]
fn variant_observation_does_not_certify_rest_struct_or_reference_patterns() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub enum Local { Some(u8), None(u16) }
        pub struct Tuple(u8);
        pub fn reference(value: Local) -> u8 {
            match value { Local::Some(ref byte) => *byte, Local::None(_) => 0 }
        }
        pub fn rest(value: Local) -> u8 {
            match value { Local::Some(..) => 1, Local::None(..) => 0 }
        }
        pub fn structure(value: Tuple) -> u8 { let Tuple(byte) = value; byte }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "excluded", "{function}");
        let facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let mut nodes = Vec::new();
        descendants(&facts.body, &mut nodes);
        if facts.name == "reference" {
            assert_eq!(
                nodes
                    .iter()
                    .filter(|n| n.kind == "pat_local_variant")
                    .count(),
                2
            );
            assert!(nodes.iter().any(|n| n.kind == "pat_binding_pattern"));
        } else {
            assert!(!nodes.iter().any(|n| n.kind == "pat_local_variant"));
            assert!(nodes.iter().any(|n| n.kind == "pat_unsupported"));
        }
        assert!(rstyle_pdg::producer::produce(&facts).is_err());
    }
}
