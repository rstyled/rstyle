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
        "conformance": {"status": "conformant", "reasons": ["old"]},
        evidence: {"contract": "old/path", "profile": "p"},
        "other": {"status": "unknown", "reasons": ["missing"], "contract": "required"}
    });
    assert_eq!(
        normalize(input, "", &BTreeMap::new()),
        json!({
            "conformance": {},
            evidence: {"profile": "p"},
            "other": {"status": "unknown", "reasons": ["missing"], "contract": "required"}
        })
    );
}

#[test]
fn cast_operand_references_rename_without_changing_cast_order_or_types() {
    let casts = serde_json::json!([
        {"pdg_node_id":"cast", "operand_pdg_node_id":"input", "operand_type":"u8", "result_type":"u16"}
    ]);
    let names = BTreeMap::from([
        ("cast".into(), "new-cast".into()),
        ("input".into(), "new-input".into()),
    ]);
    assert_eq!(
        normalize(casts, "rstyle.hir-coverage:integer-casts", &names),
        serde_json::json!([
            {"pdg_node_id":"new-cast", "operand_pdg_node_id":"new-input", "operand_type":"u8", "result_type":"u16"}
        ])
    );
}

#[test]
fn digit_origin_inventory_renames_ids_without_reordering_components() {
    let extension = "rstyle.data:digit-product-origins";
    let renamed =
        std::collections::BTreeMap::from([("z".into(), "n0".into()), ("a".into(), "n1".into())]);
    let original = serde_json::json!({"values":[
        {"pdg_node_id":"z", "value":{"components":[0,1]}},
        {"pdg_node_id":"a", "value":{"components":[1,0]}}
    ]});
    let reordered = serde_json::json!({"values":[
        {"pdg_node_id":"n1", "value":{"components":[1,0]}},
        {"pdg_node_id":"n0", "value":{"components":[0,1]}}
    ]});
    assert_eq!(
        normalize(original, extension, &renamed),
        normalize(reordered, extension, &std::collections::BTreeMap::new())
    );
}
