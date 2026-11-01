//! Matching completeness controls the denominator independently of assessed rows.
use super::*;
use serde_json::Value;

#[test]
fn matching_budget_unknown_denominator_is_distinct_from_known_empty_and_unrequested() {
    for (requested, matching_complete, execution, coverage, expected, success) in [
        (true, false, "completed", "incomplete", None, false),
        (true, true, "completed", "complete", Some(0), true),
        (false, true, "not_requested", "incomplete", None, true),
    ] {
        let report = summarize(Vec::new(), requested, matching_complete);
        assert_eq!(report.permits_success(), success);
        let report = serde_json::to_value(report).unwrap();
        assert_eq!(report["execution"], execution);
        assert_eq!(report["coverage"], coverage);
        assert_eq!(
            report["expected_pairs"],
            serde_json::to_value(expected).unwrap()
        );
        assert_eq!(report["assessed_pairs"], 0);
        assert_eq!(report["complete_pairs"], 0);
        assert_eq!(report["pairs"], Value::Array(Vec::new()));
        assert_eq!(
            report["reasons"].as_array().unwrap().is_empty(),
            !requested || matching_complete
        );
    }
}
