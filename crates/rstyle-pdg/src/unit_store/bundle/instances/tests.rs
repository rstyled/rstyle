use super::*;
use serde_json::{Value, json};
use std::cell::Cell;

fn owner() -> Arc<UnitStore> {
    Arc::new(UnitStore::capture(Vec::new()).unwrap())
}

// Deliberately uncertifiable proof inputs; decoding them cannot grant authority.
fn wire(namespace: UnitNamespace) -> Value {
    let facts: Value = serde_json::from_str(include_str!(
        "../../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    let mut projection = serde_json::Map::new();
    for key in ["source", "fn_span", "signature", "bindings", "body"] {
        projection.insert(key.into(), facts[key].clone());
    }
    projection.insert("definition".into(), json!({"path":"forged","definition_identity":"forged","crate":"forged","stable_crate_id":"forged","local":true}));
    projection.insert("arguments".into(), json!([]));
    projection.insert("comparisons".into(), json!([]));
    projection.insert("adjustments".into(), json!([]));
    projection.insert("provenance".into(), json!({"identity":"rstyle.rust.scalar-instance-provenance/1","complete":true,"parameters":[],"bounds":[],"compiler_bounds":[],"assertions":[]}));
    json!({"schema":SCHEMA,"owner_namespace":namespace,"definition_ordinal":0,"projection":projection})
}

#[test]
fn selected_owner_inspection_runs_once_before_replay() {
    let inspected = Cell::new(0);
    let original = owner();
    let session = ScalarInstanceReplay::new(Arc::clone(&original), |facts| {
        inspected.set(inspected.get() + 1);
        assert!(facts.is_empty());
        Ok(())
    })
    .unwrap();
    assert!(Arc::ptr_eq(session.owner(), &original));
    let bytes = serde_json::to_vec(&wire(*original.namespace())).unwrap();
    for _ in 0..2 {
        assert!(
            session
                .replay(&bytes)
                .err()
                .unwrap()
                .contains("absent original definition")
        );
    }
    assert_eq!(inspected.get(), 1);
    assert!(original.functions().is_empty());
}

#[test]
fn independent_context_rejection_cannot_be_replaced_by_serialized_claims() {
    let result = ScalarInstanceReplay::new(owner(), |_| Err("wrong selected context".into()));
    assert_eq!(result.err().unwrap(), "wrong selected context");
    let original = owner();
    let session = ScalarInstanceReplay::new(Arc::clone(&original), |_| Ok(())).unwrap();
    let bytes = serde_json::to_vec(&wire(*owner().namespace())).unwrap();
    assert!(
        session
            .replay(&bytes)
            .err()
            .unwrap()
            .contains("selected owner")
    );
}

#[test]
fn wire_rejects_graphs_certificates_and_nested_trust_flags() {
    let original = owner();
    let session = ScalarInstanceReplay::new(Arc::clone(&original), |_| Ok(())).unwrap();
    for key in [
        "graphs",
        "certified",
        "obligations_verified",
        "generated",
        "owner",
    ] {
        let mut forged = wire(*original.namespace());
        forged[key] = json!(true);
        assert!(
            session
                .replay(&serde_json::to_vec(&forged).unwrap())
                .err()
                .unwrap()
                .contains("unknown field")
        );
    }
    let mut forged = wire(*original.namespace());
    forged["projection"]["trusted"] = json!(true);
    assert!(
        session
            .replay(&serde_json::to_vec(&forged).unwrap())
            .err()
            .unwrap()
            .contains("unknown field")
    );
}

#[test]
fn replay_and_export_have_no_generated_policy_escape() {
    let original = owner();
    let session = crate::verification_policy::generated(|| {
        ScalarInstanceReplay::new(Arc::clone(&original), |_| Ok(()))
    })
    .unwrap();
    let mut record = wire(*original.namespace());
    record["definition_ordinal"] = json!(usize::MAX);
    let result = crate::verification_policy::generated(|| {
        session.replay(&serde_json::to_vec(&record).unwrap())
    });
    assert!(result.err().unwrap().contains("absent original definition"));
    let projection: Projection = serde_json::from_value(record["projection"].clone()).unwrap();
    assert!(
        original
            .write_scalar_instance_sidecar(Vec::new(), &projection)
            .err()
            .unwrap()
            .contains("not a scalar instance")
    );
    assert!(!crate::verification_policy::is_generated());
}
