use super::*;

#[test]
fn classifier_target_baseline_keeps_existing_family_and_records_closed_blocker_chain() {
    let directory = clone_workspace();
    let source = format!(
        "{}\n{}",
        include_str!("../../../../eval/coverage-development/tagged-units.rs"),
        include_str!("../../../../eval/coverage-development/tagged-unit-classifiers.rs")
    );
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 23);
    for function in functions {
        let previous_positive = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "Unit::new_byte" | "Unit::new_end" | "Unit::byte" | "Unit::end" | "roundtrip" | "saved"
        );
        assert_eq!(
            function["outcome"],
            if previous_positive {
                "admitted"
            } else {
                "excluded"
            },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
}
