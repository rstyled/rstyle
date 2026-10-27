//! Uncertifiable proof inputs must never mint an instance or caller authority.
use super::*;
use serde_json::json;
use std::cell::Cell;

fn owner() -> Arc<UnitStore> {
    Arc::new(UnitStore::capture(Vec::new()).unwrap())
}

fn wire(owner: &UnitStore) -> Value {
    json!({
        "schema":SCHEMA,
        "owner_namespace":owner.namespace(),
        "definition_ordinal":0,
        "haystack":"[u8]",
        "observation":{}
    })
}

#[test]
fn original_owner_is_inspected_once_and_remains_unchanged_after_failed_replay() {
    let inspected = Cell::new(0);
    let original = owner();
    let session = ReferenceConstructorReplay::new(Arc::clone(&original), |facts| {
        assert!(facts.is_empty());
        inspected.set(inspected.get() + 1);
        Ok(())
    })
    .unwrap();
    let bytes = serde_json::to_vec(&wire(&original)).unwrap();
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
    assert!(Arc::ptr_eq(session.owner(), &original));
    assert!(original.functions().is_empty());
}

#[test]
fn serialized_policy_and_graph_claims_cannot_replace_structural_proof() {
    let original = owner();
    let session = ReferenceConstructorReplay::new(Arc::clone(&original), |_| Ok(())).unwrap();
    for key in ["graphs", "certified", "generated", "trusted", "caller"] {
        let mut forged = wire(&original);
        forged[key] = json!(true);
        let error = session
            .replay(&serde_json::to_vec(&forged).unwrap())
            .err()
            .unwrap();
        assert!(error.contains("unknown field"), "{key}: {error}");
    }
}

#[test]
fn exact_owner_schema_and_closed_substitution_are_required() {
    let original = owner();
    let session = ReferenceConstructorReplay::new(Arc::clone(&original), |_| Ok(())).unwrap();
    for (field, replacement, message) in [
        (
            "schema",
            json!("rstyle.scalar-instance-sidecar/1"),
            "selected owner",
        ),
        (
            "owner_namespace",
            json!(owner().namespace()),
            "selected owner",
        ),
        ("haystack", json!("u8"), "concrete str/[u8]"),
        (
            "definition_ordinal",
            json!(usize::MAX),
            "absent original definition",
        ),
    ] {
        let mut forged = wire(&original);
        forged[field] = replacement;
        let error = session
            .replay(&serde_json::to_vec(&forged).unwrap())
            .err()
            .unwrap();
        assert!(error.contains(message), "{field}: {error}");
    }
}

#[test]
fn generated_context_does_not_enable_replay_or_export() {
    let original = owner();
    let session = ReferenceConstructorReplay::new(Arc::clone(&original), |_| Ok(())).unwrap();
    let bytes = serde_json::to_vec(&wire(&original)).unwrap();
    let replay = crate::verification_policy::generated(|| session.replay(&bytes));
    assert!(replay.err().unwrap().contains("absent original definition"));
    let export = crate::verification_policy::generated(|| {
        original.write_reference_constructor_sidecar(Vec::new())
    });
    assert!(
        export
            .err()
            .unwrap()
            .contains("not a reference constructor instance")
    );
    assert!(!crate::verification_policy::is_generated());
}

#[test]
fn independent_rejection_and_linked_owner_are_not_overridden() {
    let error = ReferenceConstructorReplay::new(owner(), |_| Err("wrong selected unit".into()));
    assert_eq!(error.err().unwrap(), "wrong selected unit");
    let mut linked = UnitStore::capture(Vec::new()).unwrap();
    linked.dependencies.push(owner());
    let error = ReferenceConstructorReplay::new(Arc::new(linked), |_| Ok(()));
    assert!(
        error
            .err()
            .unwrap()
            .contains("original unlinked definition owner")
    );
}
