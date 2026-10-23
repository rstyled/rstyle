//! Wire inputs cannot substitute for an inspected source owner or a private proof.
use super::*;
use serde_json::json;
use std::cell::Cell;

fn owner() -> Arc<UnitStore> {
    Arc::new(UnitStore::capture(Vec::new()).unwrap())
}

fn wire(owner: &UnitStore) -> Value {
    json!({
        "schema": SCHEMA,
        "owner_namespace": owner.namespace(),
        "definition_ordinal": 0,
        "argument": "Span",
        "observation": {}
    })
}

#[test]
fn independent_inspection_precedes_replay_and_missing_definitions_stay_missing() {
    let original = owner();
    let inspected = Cell::new(0);
    let replay = ProductConstructorReplay::new(Arc::clone(&original), |facts| {
        assert!(facts.is_empty());
        inspected.set(inspected.get() + 1);
        Ok(())
    })
    .unwrap();
    assert!(Arc::ptr_eq(replay.owner(), &original));
    let bytes = serde_json::to_vec(&wire(&original)).unwrap();
    for _ in 0..2 {
        let error = replay.replay(&bytes).err().unwrap();
        assert!(error.contains("absent original definition"), "{error}");
    }
    assert_eq!(inspected.get(), 1);
    let rejected = ProductConstructorReplay::new(original, |_| Err("wrong build".into()));
    assert_eq!(rejected.err().unwrap(), "wrong build");
}

#[test]
fn wrong_owner_argument_and_serialized_authority_are_rejected() {
    let original = owner();
    let replay = ProductConstructorReplay::new(Arc::clone(&original), |_| Ok(())).unwrap();
    let foreign = serde_json::to_vec(&wire(&owner())).unwrap();
    assert!(
        replay
            .replay(&foreign)
            .err()
            .unwrap()
            .contains("selected owner")
    );
    for argument in ["usize", "Span "] {
        let mut changed = wire(&original);
        changed["argument"] = argument.into();
        let bytes = serde_json::to_vec(&changed).unwrap();
        let error = replay.replay(&bytes).err().unwrap();
        assert!(!error.contains("absent original definition"), "{error}");
    }
    for key in ["graphs", "certified", "generated", "obligations_verified"] {
        let mut changed = wire(&original);
        changed[key] = true.into();
        let bytes = serde_json::to_vec(&changed).unwrap();
        assert!(
            replay
                .replay(&bytes)
                .err()
                .unwrap()
                .contains("unknown field")
        );
    }
}

#[test]
fn generated_policy_cannot_bypass_original_ownership_or_export_checks() {
    let original = owner();
    let replay = ProductConstructorReplay::new(Arc::clone(&original), |_| Ok(())).unwrap();
    let mut changed = wire(&original);
    changed["definition_ordinal"] = json!(usize::MAX);
    let bytes = serde_json::to_vec(&changed).unwrap();
    let result = crate::verification_policy::generated(|| replay.replay(&bytes));
    assert!(result.err().unwrap().contains("absent original definition"));
    let result = original.write_product_constructor_sidecar(Vec::new());
    assert!(
        result
            .err()
            .unwrap()
            .contains("not a product constructor instance")
    );
}

#[test]
fn range_wire_argument_still_requires_an_original_definition() {
    let original = owner();
    let replay = ProductConstructorReplay::new(Arc::clone(&original), |_| Ok(())).unwrap();
    let mut changed = wire(&original);
    changed["argument"] = "RangeUsize".into();
    let bytes = serde_json::to_vec(&changed).unwrap();
    assert!(
        replay
            .replay(&bytes)
            .err()
            .unwrap()
            .contains("absent original definition")
    );
}
