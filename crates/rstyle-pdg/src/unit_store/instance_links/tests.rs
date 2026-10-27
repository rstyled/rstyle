use super::*;
use serde_json::json;

fn facts() -> HirFnFacts {
    let fixtures: Value = serde_json::from_str(include_str!(
        "../../scalar_instances/provenance/fixtures.json"
    ))
    .unwrap();
    let mut facts: HirFnFacts = serde_json::from_value(fixtures[0]["facts"].clone()).unwrap();
    let requests: Vec<_> = ["i32", "u32"]
        .into_iter()
        .map(|scalar| {
            json!({
        "request":{"function":facts.name,"arguments":[scalar]},
        "status":"unsupported","reason":"original compiler-observed unsupported shape"})
        })
        .collect();
    facts.build.frontend_options.insert(
        KEY.into(),
        json!({"identity":IDENTITY,"complete":true,"requests":requests}),
    );
    facts
}

#[test]
fn complete_negative_observations_remain_distinct_and_unmodified() {
    let facts = facts();
    let original = facts.build.frontend_options.get(KEY).unwrap().clone();
    let entries = inventory(&facts).unwrap();
    assert_eq!(entries[0]["request"]["arguments"][0], "i32");
    assert_eq!(entries[1]["request"]["arguments"][0], "u32");
    assert_eq!(facts.build.frontend_options.get(KEY), Some(&original));
}

#[test]
fn all_four_known_negative_statuses_retain_their_original_reason() {
    for outcome in ["unsupported", "unresolved", "error", "unavailable"] {
        let mut facts = facts();
        let mut captured = facts.build.frontend_options.get(KEY).unwrap().clone();
        captured["requests"][0]["status"] = json!(outcome);
        facts.build.frontend_options.insert(KEY.into(), captured);
        let entries = inventory(&facts).unwrap();
        assert_eq!(entries[0]["status"], outcome);
        assert_eq!(
            entries[0]["reason"],
            "original compiler-observed unsupported shape"
        );
        assert_ne!(entries[0]["status"], "projected");
    }
}

#[test]
fn missing_duplicate_foreign_and_unknown_requests_fail_closed() {
    let original = facts();
    for pointer in [
        "/identity",
        "/requests/0/status",
        "/requests/0/request/function",
        "/requests/0/request/arguments/0",
        "/requests/0/reason",
    ] {
        let mut facts = original.clone();
        let mut inventory = facts.build.frontend_options.get(KEY).unwrap().clone();
        *inventory.pointer_mut(pointer).unwrap() = json!("");
        facts.build.frontend_options.insert(KEY.into(), inventory);
        assert!(super::inventory(&facts).is_err(), "{pointer}");
    }
    for requests in [
        json!([]),
        json!([original.build.frontend_options.get(KEY).unwrap()["requests"][0]]),
        json!([
            original.build.frontend_options.get(KEY).unwrap()["requests"][0],
            original.build.frontend_options.get(KEY).unwrap()["requests"][0]
        ]),
    ] {
        let mut facts = original.clone();
        let mut inventory = facts.build.frontend_options.get(KEY).unwrap().clone();
        inventory["requests"] = requests;
        facts.build.frontend_options.insert(KEY.into(), inventory);
        assert!(super::inventory(&facts).is_err());
    }
}

#[test]
fn partial_inventories_and_unknown_outcome_fields_cannot_become_available() {
    let mut facts = facts();
    let mut inventory = facts.build.frontend_options.get(KEY).unwrap().clone();
    inventory["complete"] = json!(false);
    facts
        .build
        .frontend_options
        .insert(KEY.into(), inventory.clone());
    assert!(super::inventory(&facts).is_err());
    inventory["complete"] = json!(true);
    inventory["requests"][0]["body"] = json!({});
    facts.build.frontend_options.insert(KEY.into(), inventory);
    assert!(super::inventory(&facts).is_err());
}
