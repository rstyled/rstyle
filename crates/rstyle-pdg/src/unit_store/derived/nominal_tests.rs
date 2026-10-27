use super::*;
use std::cell::Cell;

#[test]
fn completed_nominal_errors_are_retained_per_policy_and_owner() {
    let first: Checked<Result<(), String>> = Checked::default();
    let visits = Cell::new(0);
    let check = || {
        visits.set(visits.get() + 1);
        Ok(Err("ambiguous original source".to_owned()))
    };
    for _ in 0..3 {
        assert_eq!(
            first.get(check).unwrap().as_ref().unwrap_err(),
            "ambiguous original source"
        );
    }
    assert_eq!(visits.get(), 1);
    crate::verification_policy::generated(|| {
        assert!(first.get(check).unwrap().is_err());
        assert!(first.get(check).unwrap().is_err());
    });
    assert_eq!(visits.get(), 2);
    let other: Checked<Result<(), String>> = Checked::default();
    assert_eq!(other.get(|| Ok(Ok(()))).unwrap(), &Ok(()));
    assert!(first.get(check).unwrap().is_err());
    assert_eq!(visits.get(), 2);
}

#[test]
fn interrupted_nominal_resolution_does_not_publish_an_outcome() {
    let checked: Checked<Result<(), String>> = Checked::default();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = checked.get(|| panic!("interrupted original-source resolution"));
    }));
    assert!(panic.is_err());
    assert!(checked.0.iter().all(|slot| slot.get().is_none()));
    assert_eq!(checked.get(|| Ok(Ok(()))).unwrap(), &Ok(()));
}

fn original() -> HirFnFacts {
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../reference_instances/operations/fixtures.json"
    ))
    .unwrap();
    serde_json::from_value(fixtures[0]["canonical"].clone()).unwrap()
}

fn nominal_error(builder: &Builder) -> String {
    FunctionView {
        resolver: builder,
        id: FunctionId(0),
    }
    .nominal()
    .err()
    .unwrap()
}

#[test]
fn installed_helper_does_not_change_a_retained_original_source_failure() {
    let mut builder = Builder::new(vec![original()]).unwrap();
    let expected = nominal_error(&builder);
    assert!(builder.derived[0].nominal.0[0].get().unwrap().is_err());
    // Exercise the real installation path. This unrelated historical graph is
    // bookkeeping input only; installing it grants no original-source proof.
    builder.install(vec![historical_helper()]).unwrap();
    assert_eq!(builder.store.helpers.len(), 1);
    assert_eq!(builder.store.certified[0], Some(HelperId(0)));
    assert_eq!(nominal_error(&builder), expected);
    assert!(builder.derived[0].nominal.0[1].get().is_none());
    let view = FunctionView {
        resolver: &builder,
        id: FunctionId(0),
    };
    for _ in 0..2 {
        assert!(view.data_boundary().is_err());
    }
    assert!(
        builder.derived[0]
            .data_frontend
            .0
            .iter()
            .all(|slot| slot.get().is_none())
    );
    assert!(view.product_instance().is_none());
    assert!(view.reference_instance().is_none());
}

fn historical_helper() -> CompletedHelper {
    let summary = serde_json::from_value(serde_json::json!({
        "identity":"test:unrelated-historical-bookkeeping",
        "argument_types":[], "result_type":"unit",
        "mutates_arguments":false, "memory_effects":false,
        "may_panic":false, "always_returns":true,
        "argument_evaluation":"left-to-right", "result_dependence":"none"
    }))
    .unwrap();
    let pair = Pair {
        pdg: historical_document(),
        cfg: serde_json::from_str(include_str!(
            "../../../../../vendor-compat/cfg.example.json"
        ))
        .unwrap(),
    };
    CompletedHelper {
        function: FunctionId(0),
        summary,
        pair,
        depth: 0,
    }
}

// Same historical-schema adapter used by validate::source_text_tests. Preserve
// its explicitly unevaluated status; this document never establishes admission.
fn historical_document() -> crate::PdgDocument {
    let mut example: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vendor-compat/pdg.example.json"
    ))
    .unwrap();
    let policy: crate::ProfileId = serde_json::from_value(example["profile"].clone()).unwrap();
    example["function"]["reference_count"]["policy"] = example["profile"].clone();
    for edge in example["edges"].as_array_mut().unwrap() {
        edge["policy"] = serde_json::to_value(&policy).unwrap();
    }
    example["extraction_evidence"]["conformance"] =
        serde_json::to_value(vec![crate::Conformance::not_evaluated(
            policy,
            "Unrelated helper-table bookkeeping fixture",
        )])
        .unwrap();
    serde_json::from_value(example).unwrap()
}

#[test]
fn cached_raw_source_failure_cannot_mint_a_private_instance() {
    let builder = Builder::new(vec![original()]).unwrap();
    assert!(!nominal_error(&builder).is_empty());
    let owner = Arc::new(builder.store);
    assert!(
        UnitStore::capture_reference_constructor(
            Arc::clone(&owner),
            0,
            &serde_json::json!({"status":"verified","certified":true}),
            crate::reference_instances::Haystack::Bytes,
        )
        .is_err()
    );
    assert!(owner.reference_instance_origin().is_none());
    assert!(owner.product_instance.is_none());
}
