use super::*;
use rstyle_core::complexity::Measurement;

#[test]
fn missing_scores_remain_unavailable() {
    let facts = [sample("unmeasured")];
    let report = Report::measurements(&facts, NonZeroUsize::new(1).unwrap(), 0);
    assert_eq!(report.known_functions, 1);
    assert_eq!(report.measured_functions, 0);
    assert_eq!(report.unavailable_measurements, 1);
    assert!(!report.input_complete);
    assert!(facts[0].complexity.is_none());
}

#[test]
fn top_limit_does_not_hide_missing_scores_or_owners_in_denominators() {
    let facts = [
        FnFacts {
            complexity: Some(Measurement {
                policy_version: 1,
                cognitive: 0,
                cyclomatic: 1,
                contributions: vec![],
            }),
            ..sample("clean")
        },
        sample("missing"),
    ];
    let report = Report::measurements(&facts, NonZeroUsize::new(1).unwrap(), 3);
    assert_eq!((report.known_functions, report.measured_functions), (2, 1));
    assert_eq!(
        (report.unavailable_measurements, report.unavailable_owners),
        (1, 3)
    );
    assert!(!report.input_complete);
    let empty = Report::measurements(&[], NonZeroUsize::new(1).unwrap(), 0);
    assert!(empty.input_complete);
}

fn sample(name: &str) -> FnFacts {
    serde_json::from_value(serde_json::json!({
        "id": name, "path":"src/lib.rs", "line":1, "column":1,
        "name":name, "params":0, "has_self":false, "statements":1,
        "source":"fn go(){}", "preceding_line":"", "mutations":[],
        "calls":[], "effects":[]
    }))
    .unwrap()
}
