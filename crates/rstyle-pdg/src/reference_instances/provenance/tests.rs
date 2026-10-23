//! Independent binder expectations and adversarial omissions; no admission claims.
use super::*;
use serde_json::json;

fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!("fixtures.json")).unwrap()
}

#[test]
fn original_signature_preserves_inherited_lifetime_and_type_indexes() {
    for fixture in fixtures() {
        let facts: HirFnFacts = serde_json::from_value(fixture["original"].clone()).unwrap();
        let record = Observation::deserialize(&fixture["provenance"]).unwrap();
        definition(&facts, &record.definition).unwrap();
        types::signature(&facts, &record.signature).unwrap();
        types::arguments(&record.arguments, "[u8]", true).unwrap();
    }
}

#[test]
fn erased_instance_region_cannot_replace_original_signature_lifetime_proof() {
    for fixture in fixtures() {
        let facts: HirFnFacts = serde_json::from_value(fixture["original"].clone()).unwrap();
        for pointer in [
            "/signature/inputs/0/lifetime/index",
            "/signature/output/arguments/0/value/index",
        ] {
            let mut changed = fixture["provenance"].clone();
            *changed.pointer_mut(pointer).unwrap() = json!(1);
            let record = Observation::deserialize(&changed).unwrap();
            assert!(
                types::signature(&facts, &record.signature).is_err(),
                "{pointer}"
            );
        }
        let mut changed = fixture["provenance"].clone();
        changed["signature"]["inputs"][0]["lifetime"] = json!({"kind":"erased_region"});
        assert!(Observation::deserialize(&changed).is_err());
    }
}

#[test]
fn exact_original_parameter_and_both_concrete_bound_arguments_are_required() {
    for fixture in fixtures() {
        let owner = usize::from(
            fixture["provenance"]["owners"][0]["clauses"]
                .as_array()
                .unwrap()
                .is_empty(),
        );
        let original = &fixture["provenance"]["owners"][owner]["clauses"][1];
        let bound = model::Clause::deserialize(original).unwrap();
        types::clause(&bound, "[u8]", 1).unwrap();
        for pointer in [
            "/original_arguments/0/value/index",
            "/original_arguments/1/value/element/name",
            "/substituted_arguments/1/identity/element/name",
        ] {
            let mut changed = original.clone();
            *changed.pointer_mut(pointer).unwrap() = if pointer.ends_with("index") {
                json!(0)
            } else {
                json!("u32")
            };
            let bound = model::Clause::deserialize(&changed).unwrap();
            assert!(types::clause(&bound, "[u8]", 1).is_err(), "{pointer}");
        }
        let mut omitted = original.clone();
        omitted["original_arguments"].as_array_mut().unwrap().pop();
        assert!(types::clause(&model::Clause::deserialize(&omitted).unwrap(), "[u8]", 1).is_err());
    }
}

#[test]
fn display_text_unknown_fields_and_incomplete_structural_types_never_supply_identity() {
    let fixture = fixtures().remove(0);
    let mut changed = fixture["provenance"].clone();
    changed["arguments"][1]["value"] = json!("str");
    let record = Observation::deserialize(&changed).unwrap();
    types::arguments(&record.arguments, "[u8]", true).unwrap();
    assert!(types::arguments(&record.arguments, "str", true).is_err());
    changed["arguments"][1]["identity_status"] = json!("unsupported");
    let record = Observation::deserialize(&changed).unwrap();
    assert!(types::arguments(&record.arguments, "[u8]", true).is_err());
    changed["arguments"][1]["fabricated_authority"] = json!(true);
    assert!(Observation::deserialize(&changed).is_err());
}

#[test]
fn local_definition_display_cannot_replace_canonical_compiler_owner() {
    for fixture in fixtures() {
        let facts: HirFnFacts = serde_json::from_value(fixture["original"].clone()).unwrap();
        for field in ["definition_identity", "stable_crate_id", "crate", "path"] {
            let mut changed = fixture["provenance"]["definition"].clone();
            changed[field] = json!("another-owner");
            assert!(
                definition(&facts, &model::Definition::deserialize(&changed).unwrap()).is_err()
            );
        }
    }
}

#[test]
fn missing_or_unknown_roster_fields_fail_decoding_instead_of_becoming_empty() {
    let fixture = fixtures().remove(0);
    for pointer in ["/owners/0", "/owners/1"] {
        for field in [
            "parent",
            "parameters",
            "clauses",
            "source_predicates",
            "header",
        ] {
            let mut changed = fixture["provenance"].clone();
            changed
                .pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert!(
                Observation::deserialize(&changed).is_err(),
                "{pointer}/{field}"
            );
        }
    }
}
