use super::*;
use crate::cpg::model::Document;

/// Each function's name and the lines its facts say it waits a fixed time in a test.
fn waits_in(document: &str) -> Vec<(String, Vec<usize>)> {
    let document: Document = serde_json::from_str(document).unwrap();
    let facts = crate::cpg::rule_facts(&document).unwrap();
    facts
        .into_iter()
        .map(|facts| (facts.name, facts.test_waits))
        .collect()
}

#[test]
fn a_go_test_that_sleeps_says_where_and_other_code_says_nothing() {
    let found = waits_in(include_str!("../testdata/test-sleep-go.cpg.json"));
    assert_eq!(
        found,
        [("Backoff".into(), vec![]), ("TestReady".into(), vec![9])]
    );
}

#[test]
fn a_python_test_that_sleeps_says_where_whichever_sleep_it_awaits() {
    let mut found = waits_in(include_str!("../testdata/test-sleep-py.cpg.json"));
    found.retain(|(_, lines)| !lines.is_empty());
    assert_eq!(
        found,
        [
            ("test_ready".into(), vec![6]),
            ("test_async_ready".into(), vec![11])
        ]
    );
}

#[test]
fn every_embedded_file_reads_and_names_its_own_namespace() {
    for namespace in ["go", "py"] {
        let waits = FixedWaits::of(namespace);
        assert_eq!(waits.namespace, namespace);
        assert!(!waits.operations.is_empty(), "{namespace}");
    }
    assert!(FixedWaits::of("rust").operations.is_empty());
}
