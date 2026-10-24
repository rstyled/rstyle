use super::*;
use serde_json::json;

#[test]
fn unordered_records_preserve_duplicates_and_every_semantic_field() {
    let records = json!({"edges": [
        {"id": "transport-2", "source": "old", "target": "exit", "position": 1},
        {"id": "transport-1", "source": "old", "target": "exit", "position": 0},
        {"id": "transport-3", "source": "old", "target": "exit", "position": 0}
    ]});
    let renamed = BTreeMap::from([("old".into(), "new".into())]);
    let actual = normalize(records, "", &renamed);
    let expected = json!({"edges": [
        {"source": "new", "target": "exit", "position": 0},
        {"source": "new", "target": "exit", "position": 0},
        {"source": "new", "target": "exit", "position": 1}
    ]});
    assert_eq!(actual, expected);
    let mut missing = actual.clone();
    missing["edges"].as_array_mut().unwrap().pop();
    assert_ne!(actual, missing);
    let mut changed = actual.clone();
    changed["edges"][0]["position"] = 2.into();
    assert_ne!(actual, changed);
}

#[test]
fn ordered_values_literals_and_nested_ids_remain_evidence() {
    let renaming = BTreeMap::from([("before".into(), "after".into())]);
    let original = json!({
        "operands": ["before", "second"],
        "literal": "before",
        "nodes": [{"id": "before", "annotations": {"id": "before"}}],
        "edges": [{"id": "ephemeral", "evidence": {"id": "before"}}]
    });
    let normalized = normalize(original, "", &renaming);
    assert_eq!(
        normalized,
        json!({
            "operands": ["before", "second"],
            "literal": "before",
            "nodes": [{"id": "after", "annotations": {"id": "after"}}],
            "edges": [{"evidence": {"id": "after"}}]
        })
    );
    let mut reversed = normalized.clone();
    reversed["operands"].as_array_mut().unwrap().reverse();
    assert_ne!(normalized, normalize(reversed, "", &BTreeMap::new()));
}

#[test]
fn historical_informational_exceptions_do_not_hide_other_fields() {
    let evidence = crate::extraction_profile::EVIDENCE_KEY;
    let input = json!({
        "conformance": [{"status": "conformant", "reasons": ["old"], "profile": "p"}],
        evidence: {"contract": "old/path", "profile": "p"},
        "other": {"status": "unknown", "reasons": ["missing"], "contract": "required"}
    });
    assert_eq!(
        normalize(input, "", &BTreeMap::new()),
        json!({
            "conformance": [{"profile": "p"}],
            evidence: {"profile": "p"},
            "other": {"status": "unknown", "reasons": ["missing"], "contract": "required"}
        })
    );
}
