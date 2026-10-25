use super::*;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Inventory {
    #[serde(with = "wire_values")]
    values: BTreeMap<String, Value>,
}

fn inventory() -> Inventory {
    let value = Value {
        ty: TypeToken::Shared(Box::new(TypeToken::Scalar("u8".into()))),
        footprint: Footprint::Direct {
            origins: vec![Origin {
                parameter: 0,
                path: vec![crate::field_locations::Projection::SharedReferent],
            }],
            guards: vec![Vec::new()],
        },
    };
    Inventory {
        values: BTreeMap::from([("n1".into(), value.clone()), ("n2".into(), value)]),
    }
}

#[test]
fn origin_node_identities_are_named_wire_values_and_roundtrip_exactly() {
    let original = inventory();
    let wire = serde_json::to_value(&original).unwrap();
    let records = wire["values"].as_array().unwrap();

    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["pdg_node_id"], "n1");
    assert_eq!(records[1]["pdg_node_id"], "n2");
    assert_eq!(records[0]["value"]["ty"]["kind"], "shared");
    assert_eq!(serde_json::from_value::<Inventory>(wire).unwrap(), original);
}

#[test]
fn duplicate_wire_node_identities_cannot_overwrite_an_origin_proof() {
    let mut wire = serde_json::to_value(inventory()).unwrap();
    let records = wire["values"].as_array_mut().unwrap();
    records[1]["pdg_node_id"] = records[0]["pdg_node_id"].clone();
    let error = serde_json::from_value::<Inventory>(wire).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Duplicate digit-product origin node identity")
    );
}

#[test]
fn wire_record_extensions_cannot_hide_a_second_reference_identity() {
    let mut wire = serde_json::to_value(inventory()).unwrap();
    wire["values"][0]["node_id"] = serde_json::json!("alternate");
    assert!(serde_json::from_value::<Inventory>(wire).is_err());
}
