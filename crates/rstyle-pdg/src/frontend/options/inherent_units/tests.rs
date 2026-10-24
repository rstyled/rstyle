use super::*;
use serde_json::json;

fn value() -> Value {
    json!({"identity":crate::inherent_calls::UNIT_IDENTITY,"definitions":[{
        "definition_identity":"method", "implementation_identity":"impl",
        "implementation_span":{"start":0,"end":100},
        "owner_nominal":{"source_identity":null,"package_name":"fixture",
            "package_version":"1","manifest_digest":"manifest",
            "definition_path":"fixture::Data","declaration_digest":"declaration"},
        "source_file":"/fixture/lib.rs","function_span":{"start":10,"end":20}
    }]})
}

#[test]
fn immutable_inventory_shares_validation_and_preserves_full_json_nulls() {
    let wire = value();
    let mut first = FrontendOptions::default();
    first.insert(UNIT_KEY.into(), wire.clone());
    first.inherent_association("method").unwrap();
    let mut second = FrontendOptions::default();
    second.share_inherent_inventory(&first).unwrap();
    let (Some(Entry::InherentUnits(first_record)), Some(Entry::InherentUnits(second_record))) =
        (first.0.get(UNIT_KEY), second.0.get(UNIT_KEY))
    else {
        panic!("missing typed inventory")
    };
    assert!(Arc::ptr_eq(
        &first_record.record.value,
        &second_record.record.value
    ));
    assert!(Arc::ptr_eq(&first_record.checked, &second_record.checked));
    assert!(first_record.checked.get().is_some());
    assert_eq!(serde_json::to_value(&second).unwrap()[UNIT_KEY], wire);
}

#[test]
fn edits_replace_the_validated_snapshot_and_recheck_neighbors() {
    let mut options = FrontendOptions::default();
    options.insert(UNIT_KEY.into(), value());
    options.inherent_association("method").unwrap();
    let mut duplicate = options[UNIT_KEY]["definitions"][0].clone();
    duplicate["owner_nominal"]["definition_path"] = json!("fixture::Changed");
    options.get_mut(UNIT_KEY).unwrap()["definitions"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    options.normalize_data();
    assert!(options.inherent_association("method").is_err());
    options.insert(UNIT_KEY.into(), value());
    options.get_mut(UNIT_KEY).unwrap()["definitions"] = json!([]);
    assert!(options.inherent_association("method").is_err());
}

#[test]
fn malformed_inventory_stays_lossless_and_serialization_does_not_change_ownership() {
    let mut options = FrontendOptions::default();
    let malformed = json!({"identity":null,"definitions":"unknown"});
    options.insert(UNIT_KEY.into(), malformed.clone());
    assert!(options.inherent_association("method").is_err());
    assert_eq!(serde_json::to_value(&options).unwrap()[UNIT_KEY], malformed);
    options.insert_data(Arc::new(UnitDefinitions {
        identity: crate::inherent_calls::UNIT_IDENTITY.into(),
        definitions: vec![],
    }));
    serde_json::to_vec(&options).unwrap();
    let Some(Entry::InherentUnits(record)) = options.0.get(UNIT_KEY) else {
        panic!("missing typed inventory")
    };
    assert!(record.record.json.get().is_none());
}
