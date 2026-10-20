use super::*;

#[test]
fn borrowed_accel_source_first_baseline_preserves_ownership_boundaries() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        include_str!("../../../../eval/coverage-development/borrowed-accel-views.rs"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 18);
    for function in functions {
        let expected = if function["facts"]["name"] == "slice_window" {
            "admitted"
        } else {
            "excluded"
        };
        assert_eq!(function["outcome"], expected, "{function}");
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
