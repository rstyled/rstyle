use super::*;

#[test]
fn bitset_cardinality_source_first_baseline_keeps_complete_chain_excluded() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        include_str!("../../../../eval/coverage-development/bitset-cardinality.rs"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 14, "{report}");
    for function in functions {
        let control = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "LookSet::empty" | "LookSet::full"
        );
        assert_eq!(
            function["outcome"],
            if control { "admitted" } else { "excluded" },
            "{function}"
        );
    }
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    check_aggregate_documents(&report);
}
