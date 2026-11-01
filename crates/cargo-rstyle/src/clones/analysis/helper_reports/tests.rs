//! No rows are not evidence of a complete matching population.
use super::*;

#[test]
fn matching_budget_unknown_denominator_is_distinct_from_known_empty_and_unrequested() {
    let handoff = Handoff::new(None);
    for (requested, matching_complete, execution, coverage, expected, success) in [
        (true, false, "completed", "incomplete", None, false),
        (true, true, "completed", "complete", Some(0), true),
        (false, true, "not_requested", "incomplete", None, true),
    ] {
        let report = summarize(Vec::new(), &handoff, requested, matching_complete);
        assert_eq!(report.permits_success(), success);
        let report = serde_json::to_value(report).unwrap();
        assert_eq!(report["execution"], execution);
        assert_eq!(report["coverage"], coverage);
        assert_eq!(
            report["expected_pairs"],
            serde_json::to_value(expected).unwrap()
        );
        for field in [
            "assessed_pairs",
            "evaluated_pairs",
            "unavailable_pairs",
            "findings",
        ] {
            assert_eq!(report[field], 0);
        }
        assert!(report["pairs"].as_array().unwrap().is_empty());
        assert!(
            report["unavailable_functions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            report["reasons"].as_array().unwrap().is_empty(),
            !requested || matching_complete
        );
    }
}
