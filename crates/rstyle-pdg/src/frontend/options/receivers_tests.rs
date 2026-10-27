//! Receiver ownership preserves the open, mutable evidence boundary.
use super::*;
use crate::data_receivers::{Capture, KEY};
use serde_json::json;

fn inventory() -> Value {
    let nominal = json!({"kind":"Struct", "evidence": {
        "nominal": {"source_identity":null, "package_name":"fixture",
            "package_version":"1", "manifest_digest":"manifest",
            "definition_path":"fixture::Data", "declaration_digest":"declaration"},
        "source":"struct Data { byte: u8 }", "fields":[]
    }});
    let observed = json!({"status":"Captured", "ty":nominal});
    json!({"identity":crate::data_receivers::IDENTITY, "methods":[{
        "call":{"krate":0,"owner":1,"local":2},
        "source_span":{"start":0,"end":6},
        "receiver":{"krate":0,"owner":1,"local":3},
        "receiver_type":observed, "adjusted_type":observed,
        "adjustments":[], "signature_inputs":[observed], "signature_result":observed
    }]})
}

#[test]
fn receiver_inventory_is_borrowed_and_preserves_wire_nulls() {
    let original = inventory();
    let mut options = FrontendOptions::default();
    options.insert(KEY.into(), original.clone());
    let first = options.data::<Capture>().unwrap();
    assert!(matches!(first, Cow::Borrowed(_)));
    assert!(std::ptr::eq(
        first.as_ref(),
        options.data::<Capture>().unwrap().as_ref()
    ));
    let Some(Entry::Receivers(record)) = options.0.get(KEY) else {
        panic!("missing typed receiver inventory")
    };
    assert!(record.json.get().is_none());
    let bytes = serde_json::to_vec(&options).unwrap();
    assert_eq!(
        bytes,
        serde_json::to_vec(&BTreeMap::from([(KEY, original)])).unwrap()
    );
    let loaded: FrontendOptions = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(loaded, options);
}

#[test]
fn receiver_edits_and_invalid_records_never_reuse_previous_typed_inventory() {
    let original = inventory();
    let mut options = FrontendOptions::default();
    options.insert(KEY.into(), original.clone());
    options.get_mut(KEY).unwrap()["methods"][0]["adjustments"] = json!(false);
    assert!(options.data::<Capture>().is_err());
    options.normalize_data();
    assert!(options.data::<Capture>().is_err());
    assert_eq!(options[KEY]["methods"][0]["adjustments"], false);
    options.insert(KEY.into(), original);
    options.get_mut(KEY).unwrap()["methods"][0]["signature_inputs"] = json!([]);
    assert!(
        options.data::<Capture>().unwrap().methods[0]
            .signature_inputs
            .is_empty()
    );
    options.normalize_data();
    assert!(matches!(
        options.data::<Capture>().unwrap(),
        Cow::Borrowed(_)
    ));
    assert!(
        options.data::<Capture>().unwrap().methods[0]
            .signature_inputs
            .is_empty()
    );
    options.get_mut(KEY).unwrap()["methods"][0]["unexpected"] = json!(true);
    assert!(options.data::<Capture>().is_err());
}
