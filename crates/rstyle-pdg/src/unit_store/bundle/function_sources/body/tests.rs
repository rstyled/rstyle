use super::*;
use serde_json::{Value, json};

fn node(local: u32, children: Vec<HirNode>) -> HirNode {
    HirNode {
        hir: HirRef {
            krate: 0,
            owner: 7,
            local,
        },
        kind: format!("unsupported-{local}"),
        span: ByteSpan { start: 0, end: 3 },
        snippet: "π!".into(),
        ty: Some(ResolvedTy {
            normalized: Some("u32".into()),
            observed: "u32".into(),
            spelling: "u32".into(),
        }),
        res: Some(PathRes {
            kind: "local".into(),
            target: Some(HirRef {
                krate: 0,
                owner: 7,
                local: 0,
            }),
            description: "original".into(),
        }),
        dispatch: Some(Dispatch {
            kind: "builtin".into(),
            op: "+".into(),
            operand_tys: vec!["u32".into(), "u32".into()],
            result_ty: "u32".into(),
        }),
        expansion: Some(ExpansionSite {
            callsite: ByteSpan { start: 0, end: 3 },
            snippet: "π!".into(),
        }),
        binding: Some(format!("binding-{local}")),
        binding_mut: Some(true),
        lit: Some(local.to_string()),
        param_index: Some(local),
        is_unsafe_block: true,
        children,
    }
}

fn tree(depth: u32) -> HirNode {
    let mut body = node(0, vec![]);
    for index in 1..=depth {
        body = node(
            index,
            vec![node(index + 1000, vec![]), body, node(index + 2000, vec![])],
        );
    }
    body
}

fn store(depth: u32) -> crate::unit_store::UnitStore {
    let mut facts: crate::HirFnFacts = serde_json::from_str(include_str!(
        "../../../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    facts.body = tree(depth);
    crate::unit_store::UnitStore::capture(vec![facts]).unwrap()
}

#[test]
fn shallow_body_keeps_its_wire_shape_and_borrowed_fields() {
    let original = tree(2);
    let bytes = serde_json::to_vec(&Export(&original)).unwrap();
    assert_eq!(bytes, serde_json::to_vec(&original).unwrap());
    assert!(std::ptr::eq(
        Leaf::new(&original).snippet,
        original.snippet.as_str()
    ));
    let imported: Import = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(imported.into_node(false).unwrap(), original);
}

#[test]
fn deep_body_roundtrip_preserves_every_field_and_order_with_default_json_limits() {
    let original = tree(96);
    let recursive = serde_json::to_vec(&original).unwrap();
    assert!(serde_json::from_slice::<HirNode>(&recursive).is_err());
    let bytes = serde_json::to_vec(&Export(&original)).unwrap();
    let wire: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(wire.as_array().unwrap().len(), 289);
    let imported: Import = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(imported.into_node(true).unwrap(), original);
    let imported: Import = serde_json::from_slice(&bytes).unwrap();
    assert!(imported.into_node(false).is_err());
}

#[test]
fn flat_body_rejects_orphans_missing_children_hidden_work_and_unknown_fields() {
    let leaf = serde_json::to_value(node(1, vec![])).unwrap();
    for wire in [
        json!([]),
        json!([{"node":leaf,"children":1}]),
        json!([{"node":leaf,"children":0},{"node":leaf,"children":0}]),
        json!([{"node":leaf,"children":0,"admitted":true}]),
        json!([{"node":node(1,vec![node(2,vec![])]),"children":0}]),
    ] {
        assert!(serde_json::from_value::<Import>(wire).is_err());
    }
}

#[test]
fn deep_excluded_body_survives_strict_bundle_and_diagnostic_replay_without_admission() {
    let store = store(96);
    assert_eq!(store.graph_count(), 0);
    let mut bytes = Vec::new();
    store.write_bundle(&mut bytes).unwrap();
    let imported = crate::unit_store::UnitStore::from_bundle(&bytes).unwrap();
    assert_eq!(imported.functions(), store.functions());
    assert_eq!(imported.graph_count(), 0);
    let mut wire: Value = serde_json::from_slice(&bytes).unwrap();
    wire["schema"] = "rstyle.shared-unit/6".into();
    assert_eq!(
        crate::unit_store::UnitStore::from_bundle(&serde_json::to_vec(&wire).unwrap())
            .unwrap()
            .functions(),
        store.functions()
    );
    wire["schema"] = "rstyle.shared-unit/5".into();
    assert!(
        crate::unit_store::UnitStore::from_bundle(&serde_json::to_vec(&wire).unwrap()).is_err()
    );

    bytes.clear();
    crate::unit_store::write_diagnostic_snapshot(&mut bytes, "original", store.functions())
        .unwrap();
    let imported = crate::unit_store::read_diagnostic_snapshot(&bytes).unwrap();
    assert_eq!(imported, store.functions());
    let mut wire: Value = serde_json::from_slice(&bytes).unwrap();
    wire["schema"] = "rstyle.diagnostic.raw-facts/3".into();
    assert_eq!(
        crate::unit_store::read_diagnostic_snapshot(&serde_json::to_vec(&wire).unwrap()).unwrap(),
        store.functions()
    );
    wire["schema"] = "rstyle.diagnostic.raw-facts/2".into();
    assert!(
        crate::unit_store::read_diagnostic_snapshot(&serde_json::to_vec(&wire).unwrap()).is_err()
    );
}

#[test]
fn historical_shallow_bundles_preserve_complete_original_records() {
    let store = store(2);
    let mut bytes = Vec::new();
    store.write_bundle(&mut bytes).unwrap();
    let mut wire: Value = serde_json::from_slice(&bytes).unwrap();
    for schema in ["rstyle.shared-unit/5", "rstyle.shared-unit/4"] {
        wire["schema"] = schema.into();
        if schema == "rstyle.shared-unit/4" {
            let inventories = wire
                .as_object_mut()
                .unwrap()
                .remove("inherent_units")
                .unwrap();
            assert_eq!(inventories, json!([]));
        }
        let imported =
            crate::unit_store::UnitStore::from_bundle(&serde_json::to_vec(&wire).unwrap()).unwrap();
        assert_eq!(imported.functions(), store.functions());
    }
}

#[test]
fn headers_validate_shallow_and_deep_bodies_without_retaining_trees() {
    for depth in [2, 96] {
        let original = tree(depth);
        let bytes = serde_json::to_vec(&Export(&original)).unwrap();
        let header: Import<HeaderNode> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(header.into_node(true).unwrap(), original.hir);
        let header: Import<HeaderNode> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(header.into_node(false).is_ok(), depth < 32);
    }
}

#[test]
fn headers_preserve_full_body_field_validation() {
    let leaf = serde_json::to_value(node(1, vec![])).unwrap();
    for field in [
        "kind",
        "snippet",
        "binding",
        "lit",
        "ty",
        "res",
        "dispatch",
        "expansion",
        "span",
        "hir",
        "children",
        "is_unsafe_block",
    ] {
        let mut malformed = leaf.clone();
        malformed[field] = json!(42);
        let bytes = serde_json::to_vec(&malformed).unwrap();
        assert!(serde_json::from_slice::<Import>(&bytes).is_err(), "{field}");
        assert!(
            serde_json::from_slice::<Import<HeaderNode>>(&bytes).is_err(),
            "{field}"
        );
    }
    for wire in [
        json!([]),
        json!([{"node":leaf,"children":1}]),
        json!([{"node":leaf,"children":0},{"node":leaf,"children":0}]),
        json!([{"node":node(2,vec![node(3,vec![])]),"children":0}]),
        json!([{"node":leaf,"children":0,"extra":true}]),
    ] {
        assert!(serde_json::from_value::<Import<HeaderNode>>(wire).is_err());
    }
}

#[test]
fn headers_reject_nested_missing_duplicate_and_unknown_fields() {
    let leaf = serde_json::to_string(&node(1, vec![])).unwrap();
    for malformed in [
        leaf.replacen("\"kind\":", "\"extra\":0,\"kind\":", 1),
        leaf.replacen("\"kind\":", "\"kind\":\"duplicate\",\"kind\":", 1),
        leaf.replacen("\"kind\":\"unsupported-1\",", "", 1),
    ] {
        let nested = serde_json::to_string(&node(2, vec![]))
            .unwrap()
            .replace("\"children\":[]", &format!("\"children\":[{malformed}]"));
        assert!(serde_json::from_str::<Import>(&nested).is_err());
        assert!(serde_json::from_str::<Import<HeaderNode>>(&nested).is_err());
    }
}
