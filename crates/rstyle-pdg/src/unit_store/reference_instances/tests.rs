use super::*;
use serde_json::json;

#[test]
fn requested_constructor_must_exist_in_the_original_roster() {
    let owner = Arc::new(UnitStore::capture(Vec::new()).unwrap());
    for definition in [0, usize::MAX] {
        let error = UnitStore::capture_reference_constructor(
            Arc::clone(&owner),
            definition,
            &json!({"certified":true}),
            Haystack::Bytes,
        )
        .err()
        .unwrap();
        assert!(error.contains("absent original definition"));
    }
    assert!(owner.functions().is_empty());
    assert!(owner.reference_instance_origin().is_none());
}

#[test]
fn failed_observation_cannot_replace_the_original_owner_or_grant_an_instance() {
    let owner = Arc::new(Builder::new(vec![original()]).unwrap().store);
    let before = owner.functions.clone();
    let namespace = *owner.namespace();
    let result = UnitStore::capture_reference_constructor(
        Arc::clone(&owner),
        0,
        &json!({"admission_authority":true,"status":"verified"}),
        Haystack::Bytes,
    );
    assert!(result.is_err());
    assert_eq!(owner.functions, before);
    assert_eq!(*owner.namespace(), namespace);
    assert!(owner.reference_instance_origin().is_none());
}

#[test]
fn str_constructor_requires_the_same_complete_original_owner() {
    let owner = Arc::new(UnitStore::capture(Vec::new()).unwrap());
    let error = UnitStore::capture_reference_constructor(owner, 0, &json!({}), Haystack::Str)
        .err()
        .unwrap();
    assert_eq!(
        error,
        "Reference constructor names an absent original definition"
    );
}

fn original() -> HirFnFacts {
    let fixtures: Value = serde_json::from_str(include_str!(
        "../../reference_instances/operations/fixtures.json"
    ))
    .unwrap();
    serde_json::from_value(fixtures[0]["canonical"].clone()).unwrap()
}
