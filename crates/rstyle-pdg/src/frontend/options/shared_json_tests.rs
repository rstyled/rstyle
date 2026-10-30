use super::*;
use serde_json::json;

const KEYS: [&str; 2] = ["invocation", crate::nominal_boundary::CONTEXT_KEY];

fn owner<'a>(options: &'a FrontendOptions, key: &str) -> &'a Arc<Value> {
    let Entry::SharedJson(value) = options.0.get(key).unwrap() else {
        panic!("Expected shared immutable JSON for {key}");
    };
    value
}

#[test]
fn cloned_compiler_metadata_shares_owners_and_mutation_detaches_only_one_entry() {
    let mut original = FrontendOptions::default();
    original.insert(KEYS[0].into(), json!(["rustc", "--test"]));
    original.insert(KEYS[1].into(), json!({"package": "one", "test": true}));
    let mut changed = original.clone();
    for key in KEYS {
        assert!(Arc::ptr_eq(owner(&original, key), owner(&changed, key)));
        assert!(std::ptr::eq(
            original.get(key).unwrap(),
            &**owner(&original, key)
        ));
    }

    changed.get_mut(KEYS[0]).unwrap()[1] = json!("--crate-type=lib");
    assert!(!Arc::ptr_eq(
        owner(&original, KEYS[0]),
        owner(&changed, KEYS[0])
    ));
    assert!(Arc::ptr_eq(
        owner(&original, KEYS[1]),
        owner(&changed, KEYS[1])
    ));
    assert_eq!(original[KEYS[0]], json!(["rustc", "--test"]));
    assert_eq!(changed[KEYS[0]], json!(["rustc", "--crate-type=lib"]));
    assert_ne!(original, changed);

    changed.get_mut(KEYS[1]).unwrap()["test"] = json!(false);
    assert_eq!(original[KEYS[1]]["test"], true);
    assert!(!Arc::ptr_eq(
        owner(&original, KEYS[1]),
        owner(&changed, KEYS[1])
    ));
}

#[test]
fn compiler_metadata_roundtrips_absent_null_and_unrecognized_json_exactly() {
    for key in KEYS {
        let absent = FrontendOptions::default();
        assert_eq!(absent.get(key), None);
        for value in [
            Value::Null,
            json!([]),
            json!(17),
            json!({"unknown": [null, true]}),
        ] {
            let mut options = FrontendOptions::default();
            options.insert(key.into(), value.clone());
            assert_ne!(options, absent);
            let wire = serde_json::to_value(&options).unwrap();
            assert_eq!(wire, json!({key: value}));
            let restored: FrontendOptions = serde_json::from_value(wire).unwrap();
            assert_eq!(restored, options);
            assert!(!Arc::ptr_eq(owner(&restored, key), owner(&options, key)));
            assert_eq!(restored.get(key), Some(&value));
            assert_eq!(options.clone().remove(key), Some(value.clone()));
            assert_eq!(options.remove(key), Some(value));
            assert_eq!(options, absent);
        }
    }
}

#[test]
fn context_values_remain_distinct_and_other_options_keep_ordinary_json() {
    let mut options = FrontendOptions::default();
    options.insert(KEYS[1].into(), json!({"target": "lib", "test": false}));
    options.insert("custom".into(), json!({"unknown": null}));
    assert!(matches!(options.0.get("custom"), Some(Entry::Json(_))));
    let mut other = options.clone();
    other.insert(KEYS[1].into(), json!({"target": "lib", "test": true}));
    assert_ne!(options, other);
    assert_eq!(
        other.insert(KEYS[1].into(), json!({"target": "lib", "test": false})),
        Some(json!({"target": "lib", "test": true}))
    );
    assert_eq!(options, other);
    assert!(!Arc::ptr_eq(
        owner(&options, KEYS[1]),
        owner(&other, KEYS[1])
    ));
    let wire = serde_json::to_value(&options).unwrap();
    assert_eq!(wire["custom"], json!({"unknown": null}));
}
