use super::*;
use serde_json::json;

fn options(count: usize) -> FrontendOptions {
    let definitions: Vec<_> = (0..count)
        .map(|index| {
            json!({
                "definition_identity":format!("method{index}"), "implementation_identity":"impl",
                "implementation_span":{"start":0,"end":10000},
                "owner_nominal":{"package_name":"fixture","package_version":"1",
                    "manifest_digest":"manifest","definition_path":"fixture::Data",
                    "declaration_digest":"declaration"},
                "source_file":"/fixture/lib.rs",
                "function_span":{"start":index*10,"end":index*10+5}
            })
        })
        .collect();
    let mut options = FrontendOptions::default();
    options.insert(
        crate::inherent_calls::UNIT_KEY.into(),
        json!({
            "identity":crate::inherent_calls::UNIT_IDENTITY,"definitions":definitions
        }),
    );
    options
}

#[test]
fn inventory_transport_shares_distinct_equal_owners_and_grows_with_references() {
    let first = options(128);
    let equal = options(128);
    let mut table = Inventories::default();
    let mut sources = source_table::Owners::default();
    let mut references = Vec::new();
    for index in 0..128 {
        let original = if index % 2 == 0 { &first } else { &equal };
        references.push(
            serde_json::to_value(Export::new(original, &mut sources, &mut table).unwrap()).unwrap(),
        );
    }
    let inventory_wire = serde_json::to_value(&table).unwrap();
    assert_eq!(inventory_wire.as_array().unwrap().len(), 1);
    assert!(serde_json::to_vec(&references).unwrap().len() < 128 * 128);
    let mut imported =
        ImportedInventories::new(Some(inventory_wire.as_array().unwrap().clone()), false).unwrap();
    let mut source_imports = source_table::Imported::new(&[]);
    let mut owners = Vec::new();
    for wire in references {
        let options = serde_json::from_value::<Import>(wire)
            .unwrap()
            .decode(&mut source_imports, &mut imported)
            .unwrap();
        owners.push(
            options
                .data::<Arc<crate::inherent_calls::UnitDefinitions>>()
                .unwrap()
                .into_owned(),
        );
    }
    imported.finish().unwrap();
    assert!(owners.iter().all(|owner| Arc::ptr_eq(&owners[0], owner)));
}

#[test]
fn tables_reject_missing_dangling_unused_duplicate_and_contradictory_entries() {
    let original = options(2)[crate::inherent_calls::UNIT_KEY].clone();
    assert!(ImportedInventories::new(None, false).is_err());
    assert!(ImportedInventories::new(Some(vec![]), true).is_err());
    let mut empty = ImportedInventories::new(Some(vec![]), false).unwrap();
    assert!(empty.entry(0).is_err());
    let unused = ImportedInventories::new(Some(vec![original.clone()]), false).unwrap();
    assert!(unused.finish().is_err());
    assert!(
        ImportedInventories::new(Some(vec![original.clone(), original.clone()]), false).is_err()
    );
    let mut changed = original.clone();
    changed["definitions"][0]["owner_nominal"]["definition_path"] = json!("fixture::Other");
    assert!(ImportedInventories::new(Some(vec![original.clone(), changed]), false).is_err());
    let mut contradictory = original;
    contradictory["definitions"][1]["definition_identity"] = json!("method0");
    assert!(ImportedInventories::new(Some(vec![contradictory]), false).is_err());
}

#[test]
fn historical_inline_and_malformed_records_do_not_get_interpreted_as_references() {
    let original = options(2);
    let mut imported = ImportedInventories::new(None, true).unwrap();
    let inline = serde_json::to_value(&original).unwrap();
    let mut sources = source_table::Imported::new(&[]);
    let restored = serde_json::from_value::<Import>(inline)
        .unwrap()
        .decode(&mut sources, &mut imported)
        .unwrap();
    assert_eq!(restored, original);
    let mut malformed = serde_json::to_value(&original).unwrap();
    malformed[crate::inherent_calls::UNIT_KEY] = json!({"kind":"inherent_unit","value":0});
    let restored = serde_json::from_value::<Import>(malformed.clone())
        .unwrap()
        .decode(&mut sources, &mut imported)
        .unwrap();
    assert_eq!(serde_json::to_value(restored).unwrap(), malformed);
    imported.finish().unwrap();
}
