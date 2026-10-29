use super::*;
use crate::clones::review::{ReviewStore, compiler_owners::Signature, exact_rules::ReviewSnapshot};

fn snapshot() -> Snapshot {
    let context: local::BuildContext = serde_json::from_value(serde_json::json!({
        "compiler_commit":"compiler", "edition":"2024", "data_layout":"layout",
        "compilation_target":{"name":"sample","kind":["lib"],"test":false},
        "target_triple":"aarch64-unknown-linux-gnu", "pointer_width":64,
        "features":[], "cfgs":["unix"], "opt_level":"0", "debug_assertions":true,
        "overflow_checks":true, "panic_strategy":"unwind", "extraction_policy":"hir-data/1"
    }))
    .unwrap();
    let signature: Signature = serde_json::from_value(serde_json::json!({
        "parameters":[], "result":{"normalized":"u128","observed":"u128","spelling":"u128"},
        "method":null, "abi":{"status":"known","value":"Rust"}, "generics":[],
        "has_self":false, "is_variadic":false, "is_async":false, "is_unsafe":false, "is_const":false
    }))
    .unwrap();
    Snapshot {
        owner: Rc::new(Owner {
            locator: serde_json::from_value(serde_json::json!({
                "package":{"identity":"Cargo.toml","name":"sample","version":"1.0.0"},
                "source":"src/lib.rs", "qualified_name":"sample::original",
                "declaration_kind":"compiler-function", "discriminator":"unique-declaration"
            }))
            .unwrap(),
            context,
            signature,
        }),
        versions: local_adapter::versions("compiler-diagnostics"),
        metrics: rstyle_core::advisory::METRICS.into(),
        policy: Rc::new(serde_json::to_value(rstyle_core::config::Config::default()).unwrap()),
        rule_id: "fn_params".into(),
        actual: 5,
        limit: 4,
        details: vec![],
    }
}

#[test]
fn saved_diagnostics_share_owner_and_policy_without_copying_bodies() {
    let original = snapshot();
    let mut entries = Vec::new();
    exact_rules::record_with_reason(
        &mut entries,
        &exact_rules::LiveFinding::checked(&original).unwrap(),
        Some("Reviewed fixture"),
    )
    .unwrap();
    assert!(Rc::ptr_eq(&original.owner, &entries[0].finding.owner));
    assert!(Rc::ptr_eq(&original.policy, &entries[0].finding.policy));
    let store = ReviewStore {
        schema_version: 9,
        compiler_diagnostic_entries: entries,
        ..ReviewStore::default()
    };
    let wire = store.to_json().unwrap();
    for absent in ["sha256", "graph", "fingerprint", "diagnostic_index"] {
        assert!(!wire.contains(absent));
    }
    assert_eq!(ReviewStore::from_json(&wire).unwrap(), store);
}

#[test]
fn separate_rules_on_the_same_owner_never_reuse_or_stale_each_other() {
    let original = snapshot();
    let entries = vec![Entry {
        finding: original.clone(),
        reason: "Reviewed fixture".into(),
    }];
    let checked = exact_rules::ValidatedStore::new(&entries).unwrap();
    let mut other = original;
    other.rule_id = "fn_length".into();
    other.policy = Rc::new(serde_json::json!({"changed":"policy"}));
    let current = exact_rules::LiveFinding::checked(&other).unwrap();
    assert!(matches!(
        checked.lookup(&current),
        exact_rules::Lookup::VisibleNew
    ));
    other.rule_id = "fn_params".into();
    let current = exact_rules::LiveFinding::checked(&other).unwrap();
    assert!(matches!(
        checked.lookup(&current),
        exact_rules::Lookup::Stale(_)
    ));
}

#[test]
fn historical_metrics_are_readable_but_cannot_establish_current_negatives() {
    use crate::clones::review::{compiler_diagnostics, numeric_workflow::Retained, status};
    let mut previous = snapshot();
    previous.metrics = rstyle_core::advisory::LEGACY_METRICS.into();
    assert!(previous.validate().is_ok());
    assert_eq!(
        previous.changed(&snapshot()),
        Some(exact_rules::StaleReason::Policy)
    );
    let assessed = compiler_diagnostics::observed::Assessed {
        owner: Ok(Rc::clone(&previous.owner)),
        path: "src/lib.rs".into(),
        suppression: Some(rstyle_core::suppress::Suppression::None),
        findings: Default::default(),
    };
    let mut current = compiler_diagnostics::retained::Row {
        assessed: &assessed,
        snapshots: &[],
        policy: &previous.policy,
        metrics: rstyle_core::advisory::METRICS,
        selected: true,
        effect_scope_complete: true,
    };
    assert_eq!(current.assess(&previous).0, status::State::Unavailable);
    current.metrics = rstyle_core::advisory::LEGACY_METRICS;
    assert_eq!(current.assess(&previous).0, status::State::NoFinding);
}

#[test]
fn complete_fields_metrics_versions_and_visible_threshold_are_required() {
    let original = snapshot();
    let wire = serde_json::to_value(&original).unwrap();
    for field in [
        "owner", "metrics", "policy", "versions", "rule_id", "actual", "limit", "details",
    ] {
        let mut changed = wire.clone();
        changed.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<Snapshot>(changed).is_err(),
            "{field}"
        );
    }
    let mut variants = vec![original; 4];
    variants[0].metrics = "future/9".into();
    variants[1].versions.detector_version += 1;
    variants[2].actual = variants[2].limit;
    variants[3].rule_id = "invalid_suppression".into();
    assert!(variants.iter().all(|snapshot| snapshot.validate().is_err()));
}

#[test]
fn completed_status_prefers_exact_context_independently_of_row_order() {
    use crate::clones::review::{positive_workflow, status};
    struct Current<'a>(&'a Result<Snapshot, String>);
    impl positive_workflow::Row for Current<'_> {
        type Snapshot = Snapshot;
        fn snapshot(&self) -> &Result<Snapshot, String> {
            self.0
        }
    }
    let original = snapshot();
    let entries = vec![Entry {
        finding: original.clone(),
        reason: "Fixture choice".into(),
    }];
    let exact = Ok(original.clone());
    let mut changed = original.clone();
    Rc::make_mut(&mut changed.owner).context.overflow_checks = false;
    let different_context = Ok(changed);
    let mut changed = original;
    changed.actual += 1;
    let different_count = Ok(changed);
    for snapshots in [
        [&exact, &different_context, &different_count],
        [&different_count, &different_context, &exact],
    ] {
        let rows: Vec<_> = snapshots.into_iter().map(Current).collect();
        let evaluated = positive_workflow::evaluate(
            &entries,
            &rows,
            positive_workflow::Contract {
                retain_records: true,
                requested: true,
                family: status::Family::CompilerDiagnostic,
                label: "Fixture",
            },
        )
        .unwrap();
        assert_eq!(evaluated.records[0].state, status::State::HiddenExact);
    }
}
