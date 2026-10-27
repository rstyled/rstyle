//! Transport claims cannot substitute for either complete original proof.
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
        "haystack": "[u8]",
        "observation": {},
        "constructor": {
            "schema": "rstyle.reference-constructor-sidecar/1",
            "owner_namespace": owner.namespace(),
            "definition_ordinal": 0,
            "haystack": "[u8]",
            "observation": {}
        }
    })
}

#[test]
fn both_replays_share_one_independently_inspected_immutable_owner() {
    let original = owner();
    let inspected = Cell::new(0);
    let constructor = ReferenceConstructorReplay::new(Arc::clone(&original), |facts| {
        assert!(facts.is_empty());
        inspected.set(inspected.get() + 1);
        Ok(())
    })
    .unwrap();
    let session = ReferenceForwarderReplay::from_constructor(constructor);
    for _ in 0..2 {
        let error = session.replay(&serde_json::to_vec(&wire(&original)).unwrap());
        assert!(error.err().unwrap().contains("absent original definition"));
    }
    assert_eq!(inspected.get(), 1);
    assert!(Arc::ptr_eq(session.owner(), &original));
    assert!(original.functions().is_empty());
}

#[test]
fn graphs_policy_and_call_claims_are_rejected_in_both_transport_layers() {
    let original = owner();
    let session = ReferenceForwarderReplay::new(Arc::clone(&original), |_| Ok(())).unwrap();
    for nested in [false, true] {
        for key in [
            "graphs",
            "certified",
            "generated",
            "trusted",
            "caller",
            "call_link",
        ] {
            let mut forged = wire(&original);
            let layer = if nested {
                &mut forged["constructor"]
            } else {
                &mut forged
            };
            layer[key] = json!(true);
            let error = session
                .replay(&serde_json::to_vec(&forged).unwrap())
                .err()
                .unwrap();
            assert!(error.contains("unknown field"), "{nested}/{key}: {error}");
        }
    }
}

#[test]
fn both_layers_require_the_exact_schema_owner_and_closed_substitution() {
    let original = owner();
    let session = ReferenceForwarderReplay::new(Arc::clone(&original), |_| Ok(())).unwrap();
    for nested in [false, true] {
        for (field, replacement, message) in [
            ("schema", json!("unknown/1"), "selected owner"),
            (
                "owner_namespace",
                json!(owner().namespace()),
                "selected owner",
            ),
            ("haystack", json!("u8"), "concrete str/[u8]"),
        ] {
            let mut forged = wire(&original);
            let layer = if nested {
                &mut forged["constructor"]
            } else {
                &mut forged
            };
            layer[field] = replacement;
            let error = session
                .replay(&serde_json::to_vec(&forged).unwrap())
                .err()
                .unwrap();
            assert!(error.contains(message), "{nested}/{field}: {error}");
        }
    }
}

#[test]
fn generated_policy_cannot_mint_a_forwarder_or_export_an_original_owner() {
    let original = owner();
    let session = ReferenceForwarderReplay::new(Arc::clone(&original), |_| Ok(())).unwrap();
    let bytes = serde_json::to_vec(&wire(&original)).unwrap();
    let replay = crate::verification_policy::generated(|| session.replay(&bytes));
    assert!(replay.err().unwrap().contains("absent original definition"));
    let export = crate::verification_policy::generated(|| {
        original.write_reference_forwarder_sidecar(Vec::new())
    });
    assert!(export.err().unwrap().contains("not a reference forwarder"));
    assert!(!crate::verification_policy::is_generated());
}

#[test]
fn independently_rejected_and_linked_owners_remain_unusable() {
    let error = ReferenceForwarderReplay::new(owner(), |_| Err("wrong selected unit".into()));
    assert_eq!(error.err().unwrap(), "wrong selected unit");
    let mut linked = UnitStore::capture(Vec::new()).unwrap();
    linked.dependencies.push(owner());
    let error = ReferenceForwarderReplay::new(Arc::new(linked), |_| Ok(()));
    assert!(
        error
            .err()
            .unwrap()
            .contains("original unlinked definition owner")
    );
}
