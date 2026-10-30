use super::*;
use serde_json::json;

fn encode(options: &[FrontendOptions], shared: bool) -> Vec<Value> {
    let mut sources = source_table::Owners::default();
    let mut inventories = if shared {
        Inventories::with_shared_options()
    } else {
        Inventories::default()
    };
    options
        .iter()
        .map(|options| {
            serde_json::to_value(
                super::super::Export::new(options, &mut sources, &mut inventories).unwrap(),
            )
            .unwrap()
        })
        .collect()
}

fn decode(values: &[Value], shared: bool) -> Result<Vec<FrontendOptions>, String> {
    let mut sources = source_table::Imported::new(&[]);
    let mut inventories = ImportedInventories::new(Some(vec![]), false)?;
    inventories.share_options(shared);
    values
        .iter()
        .map(|value| {
            let import: super::super::Import =
                serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
            import.decode(&mut sources, &mut inventories)
        })
        .collect()
}

fn metadata(value: Value) -> FrontendOptions {
    let mut options = FrontendOptions::default();
    for key in ["invocation", crate::nominal_boundary::CONTEXT_KEY] {
        options.insert(key.into(), value.clone());
    }
    options
}

#[test]
fn references_share_equal_independent_values_and_keep_keys_and_distinct_values() {
    let first = metadata(json!({"args":["--check"], "unknown":null}));
    let equal = metadata(json!({"args":["--check"], "unknown":null}));
    let different = metadata(json!({"args":["--test"]}));
    let original = vec![first.clone(), equal, different, first];
    let wire = encode(&original, true);
    let context = crate::nominal_boundary::CONTEXT_KEY;
    assert_eq!(wire[0]["invocation"]["kind"], "inline");
    assert_eq!(wire[0][context]["kind"], "inline");
    assert_eq!(
        wire[1]["invocation"],
        json!({"kind":"shared_reference", "value":0})
    );
    assert_eq!(
        wire[1][context],
        json!({"kind":"shared_reference", "value":1})
    );
    assert_eq!(wire[2]["invocation"]["kind"], "inline");
    let restored = decode(&wire, true).unwrap();
    assert_eq!(restored, original);
    for key in ["invocation", context] {
        assert!(std::ptr::eq(
            restored[0].get(key).unwrap(),
            restored[1].get(key).unwrap()
        ));
        assert!(std::ptr::eq(
            restored[0].get(key).unwrap(),
            restored[3].get(key).unwrap()
        ));
        assert!(!std::ptr::eq(
            restored[0].get(key).unwrap(),
            restored[2].get(key).unwrap()
        ));
    }
    assert!(!std::ptr::eq(
        restored[0].get("invocation").unwrap(),
        restored[0].get(context).unwrap()
    ));
}

#[test]
fn ordinary_codecs_and_absent_null_or_malformed_metadata_stay_lossless() {
    let mut absent = FrontendOptions::default();
    absent.insert(
        "other".into(),
        json!({"kind":"shared_reference", "value":999}),
    );
    let originals = vec![
        absent,
        metadata(Value::Null),
        metadata(json!(["malformed"])),
        metadata(json!({"kind":"shared_reference", "value":999})),
    ];
    for shared in [false, true] {
        let wire = encode(&originals, shared);
        assert!(wire[0].get("invocation").is_none());
        assert_eq!(decode(&wire, shared).unwrap(), originals);
        if !shared {
            for (options, value) in originals.iter().zip(wire) {
                assert_eq!(serde_json::to_value(options).unwrap(), value);
            }
        }
    }
}

#[test]
fn malformed_forward_out_of_range_and_wrong_key_or_kind_references_reject() {
    let original = vec![metadata(json!({"args":[]})); 2];
    let wire = encode(&original, true);
    for (index, key, value) in [
        (
            0,
            "invocation",
            json!({"kind":"shared_reference", "value":0}),
        ),
        (
            1,
            "invocation",
            json!({"kind":"shared_reference", "value":999}),
        ),
        (
            1,
            "invocation",
            json!({"kind":"shared_reference", "value":1}),
        ),
        (
            1,
            "invocation",
            json!({"kind":"shared_reference", "value":-1}),
        ),
        (
            1,
            "invocation",
            json!({"kind":"census_reference", "value":0}),
        ),
        (
            1,
            "invocation",
            json!({"kind":"shared_reference", "value":0, "extra":true}),
        ),
        (
            1,
            crate::nominal_boundary::census::KEY,
            json!({"kind":"shared_reference", "value":0}),
        ),
    ] {
        let mut changed = wire.clone();
        changed[index][key] = value;
        assert!(decode(&changed, true).is_err(), "{changed:?}");
    }
}
