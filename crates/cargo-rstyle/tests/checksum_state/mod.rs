//! Connected source-selected checksum state, not general trait support.
use super::*;
mod corruption;
mod targets;

mod oracle {
    include!("../../../../eval/coverage-development/checksum-state.rs");
    include!("../../../../eval/coverage-development/checksum-state-oracle.rs");
}

#[test]
fn checksum_state_roundtrip_family_completes_the_strict_pipeline() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        include_str!("../../../../eval/coverage-development/checksum-state.rs"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 11);
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        let positive = matches!(
            name,
            "state::State::from_checksum"
                | "state::State::checksum"
                | "roundtrip"
                | "saved_input"
                | "widen"
        );
        assert_eq!(
            function["outcome"],
            if positive { "admitted" } else { "excluded" },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_evaluation_complete",
        "pair_enumeration_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    let pairs = report["matching"]["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 10);
    assert!(
        pairs
            .iter()
            .all(|pair| pair["evaluation_status"] == "evaluated")
    );
}
