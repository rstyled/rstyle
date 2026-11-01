use super::*;

use crate::unit_store::tests::empty_function_store;

fn receipt(store: &UnitStore, generated: bool) -> Arc<ValidatedIndexes> {
    let analysis = store.analysis();
    let function = analysis.function(0).unwrap();
    if generated {
        crate::verification_policy::generated(|| function.verified_pair().unwrap().into_indexes())
    } else {
        crate::verification_policy::strict(|| function.verified_pair().unwrap().into_indexes())
    }
}

#[test]
fn immutable_owner_reuses_validation_across_analyses_and_partitions_policy() {
    let store = empty_function_store();
    let generated = receipt(&store, true);
    assert!(Arc::ptr_eq(&generated, &receipt(&store, true)));

    let strict = receipt(&store, false);
    assert!(!Arc::ptr_eq(&generated, &strict));
    assert!(Arc::ptr_eq(&strict, &receipt(&store, false)));
    assert_eq!(store.validations.0.lock().unwrap().len(), 2);
}

#[test]
fn generated_receipt_cannot_bypass_strict_evidence_or_independent_verification() {
    let mut store = empty_function_store();
    store.graphs[0]
        .pdg
        .extensions
        .insert("test:additional-evidence".into(), serde_json::json!(true));
    receipt(&store, true);
    let analysis = store.analysis();
    let function = analysis.function(0).unwrap();
    assert!(function.verified_pair().is_err());
    assert!(function.verified_pair().is_err());
    assert_eq!(store.validations.0.lock().unwrap().len(), 1);

    let mut detached = store.pair(0).unwrap().clone();
    detached.cfg.entry_node_id = "absent".into();
    assert!(crate::verification_policy::generated(|| {
        crate::producer::verify(&detached, &function).is_err()
    }));
    let mut bytes = Vec::new();
    store.write_bundle(&mut bytes).unwrap();
    assert!(crate::verification_policy::generated(|| {
        UnitStore::from_bundle(&bytes).is_err()
    }));
}

#[test]
fn current_run_load_retains_receipts() {
    let store = empty_function_store();
    let mut bytes = Vec::new();
    let manifest = store.write_sealed_bundle(&mut bytes).unwrap();
    let bundle = manifest.verify_owned(bytes).unwrap();
    let loaded = UnitStore::from_verified_current_run(&bundle, |_| Ok(())).unwrap();

    assert_eq!(loaded.validations.0.lock().unwrap().len(), 1);
    let generated = receipt(&loaded, true);
    assert!(Arc::ptr_eq(&generated, &receipt(&loaded, true)));
}
