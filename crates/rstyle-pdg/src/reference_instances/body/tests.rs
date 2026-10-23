//! Retained compiler rosters and independent saved-value/constructor expectations.
use super::*;
use serde_json::json;

fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!("fixtures.json")).unwrap()
}

fn canonical(index: usize, fixture: &Value) -> HirFnFacts {
    let operations: Value =
        serde_json::from_str(include_str!("../operations/fixtures.json")).unwrap();
    let mut facts: HirFnFacts =
        serde_json::from_value(operations[index]["canonical"].clone()).unwrap();
    facts.signature = serde_json::from_value(fixture["canonical_signature"].clone()).unwrap();
    facts.bindings = serde_json::from_value(fixture["canonical_bindings"].clone()).unwrap();
    facts.body = serde_json::from_value(fixture["canonical_body"].clone()).unwrap();
    for (key, field) in [
        (crate::concrete_types::KEY, "interface"),
        (crate::data_receivers::KEY, "receivers"),
        (crate::data_library::KEY, "libraries"),
    ] {
        facts
            .build
            .frontend_options
            .insert(key.into(), fixture[field].clone());
    }
    facts
}

fn observation(index: usize, fixture: &Value) -> Value {
    let provenance: Value =
        serde_json::from_str(include_str!("../provenance/fixtures.json")).unwrap();
    let operations: Value =
        serde_json::from_str(include_str!("../operations/fixtures.json")).unwrap();
    let original = &operations[index]["canonical"];
    let mut proof = provenance[0]["provenance"].clone();
    proof["arguments"] = fixture["arguments"].clone();
    let haystack = fixture["haystack"].as_str().unwrap();
    for owner in proof["owners"].as_array_mut().unwrap() {
        for clause in owner["clauses"].as_array_mut().unwrap() {
            clause["substituted_arguments"][0] = fixture["arguments"][1].clone();
        }
    }
    json!({"request":{"function":original["name"],"haystack":haystack},"status":"observed",
        "definition":proof["definition"],"provenance":proof,"arguments":fixture["arguments"],
        "original":original,"projected_result":fixture["projected_result"],"nodes":fixture["nodes"],
        "conversions":operations[index]["conversions"],"calls":[]})
}

#[test]
fn complete_constructor_source_has_exact_independent_value_and_binding_rosters() {
    for (index, fixture) in fixtures().iter().enumerate() {
        let facts = canonical(index, fixture);
        let checked = source::verify(&facts).unwrap();
        assert_eq!(checked.types.len(), 13);
        assert_eq!(facts.bindings.len(), 2);
        assert_eq!(
            checked
                .types
                .values()
                .filter(|role| **role == Role::Formal)
                .count(),
            2
        );
        assert_eq!(
            checked
                .types
                .values()
                .filter(|role| **role == Role::Bytes)
                .count(),
            4
        );
        assert_eq!(
            checked
                .types
                .values()
                .filter(|role| **role == Role::Input)
                .count(),
            2
        );
        library::verify(&facts, checked.length).unwrap();
        let record = model::Observation::deserialize(&observation(index, fixture)).unwrap();
        projection::verify(
            &facts,
            &record,
            &checked.types,
            fixture["haystack"].as_str().unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn saved_view_receiver_may_not_be_replaced_by_the_shadowed_formal() {
    for (index, fixture) in fixtures().iter().enumerate() {
        for child in [0, 1] {
            let mut facts = canonical(index, fixture);
            let formal = facts.signature.params[0].pat.hir.clone();
            let read = if child == 0 {
                &mut facts.body.children[1].children[0]
            } else {
                &mut facts.body.children[1].children[1].children[1].children[0]
            };
            read.res.as_mut().unwrap().target = Some(formal);
            assert!(source::verify(&facts).is_err());
        }
    }
}

#[test]
fn source_and_hir_reject_extra_hidden_work_missing_children_and_binding_aliases() {
    let fixture = fixtures().remove(0);
    let facts = canonical(0, &fixture);
    let mut hidden = facts.clone();
    hidden.body.children.push(facts.body.children[0].clone());
    assert!(source::verify(&hidden).is_err());
    let mut omitted = facts.clone();
    omitted.body.children[1].children.pop();
    assert!(source::verify(&omitted).is_err());
    let mut conflated = facts.clone();
    conflated.bindings[1].hir = conflated.bindings[0].hir.clone();
    assert!(source::verify(&conflated).is_err());
    let mut invented = facts.clone();
    invented.bindings.push(facts.bindings[1].clone());
    assert!(source::verify(&invented).is_err());
}

#[test]
fn projected_type_node_roster_rejects_omissions_duplicates_and_wrong_substitutions() {
    for (index, fixture) in fixtures().iter().enumerate() {
        let facts = canonical(index, fixture);
        let checked = source::verify(&facts).unwrap();
        let original = observation(index, fixture);
        for mode in [
            "missing",
            "duplicate",
            "formal",
            "function_item",
            "result_lifetime",
        ] {
            let mut changed = original.clone();
            match mode {
                "missing" => {
                    changed["nodes"].as_array_mut().unwrap().pop();
                }
                "duplicate" => changed["nodes"][1] = changed["nodes"][0].clone(),
                "formal" => {
                    changed["nodes"][0]["ty"][0]["identity"] = json!({"kind":"builtin","name":"u8"})
                }
                "function_item" => changed["nodes"][0]["kind"] = json!("function_item"),
                _ => changed["projected_result"][0]["identity"]["arguments"] = json!([]),
            }
            let record = model::Observation::deserialize(&changed).unwrap();
            assert!(
                projection::verify(
                    &facts,
                    &record,
                    &checked.types,
                    fixture["haystack"].as_str().unwrap()
                )
                .is_err(),
                "{mode}"
            );
        }
    }
}

#[test]
fn constructor_constants_and_complete_field_names_are_not_inferred_from_types() {
    let fixture = fixtures().remove(0);
    let facts = canonical(0, &fixture);
    for (target, literal) in [("start", "1"), ("earliest", "true")] {
        let mut changed = facts.clone();
        let node = if target == "start" {
            &mut changed.body.children[1].children[1].children[0]
        } else {
            &mut changed.body.children[1].children[3]
        };
        node.lit = Some(literal.into());
        assert!(source::verify(&changed).is_err());
    }
    let mut reordered = facts.clone();
    reordered.body.children[1].children.swap(0, 1);
    assert!(source::verify(&reordered).is_err());
}

#[test]
fn length_evidence_requires_complete_adjustments_and_exact_registered_definition() {
    let fixture = fixtures().remove(0);
    for mode in ["adjustment", "callee", "missing_record"] {
        let mut facts = canonical(0, &fixture);
        if mode == "callee" {
            let length = &mut facts.body.children[1].children[1].children[1];
            let mut callee: Value =
                serde_json::from_str(&length.res.as_ref().unwrap().description).unwrap();
            callee["definition_hash"] = json!("fabricated");
            length.res.as_mut().unwrap().description = callee.to_string();
        } else {
            let mut receivers = fixture["receivers"].clone();
            if mode == "adjustment" {
                receivers["methods"][1]["adjustments"]
                    .as_array_mut()
                    .unwrap()
                    .reverse();
            } else {
                receivers["methods"].as_array_mut().unwrap().pop();
            }
            facts
                .build
                .frontend_options
                .insert(crate::data_receivers::KEY.into(), receivers);
        }
        let checked = source::verify(&facts).unwrap();
        assert!(library::verify(&facts, checked.length).is_err(), "{mode}");
    }
}
