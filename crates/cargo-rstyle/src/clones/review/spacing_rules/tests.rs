use super::*;
use crate::clones::review::{CompilationTarget, Package};

fn context() -> local::BuildContext {
    local::BuildContext {
        compiler_commit: "compiler".into(),
        edition: "2024".into(),
        data_layout: "layout".into(),
        compilation_target: CompilationTarget {
            name: "sample".into(),
            kind: vec!["lib".into()],
            test: false,
        },
        target_triple: "aarch64-unknown-linux-gnu".into(),
        pointer_width: 64,
        features: vec![],
        cfgs: vec!["unix".into()],
        opt_level: "0".into(),
        debug_assertions: true,
        overflow_checks: true,
        panic_strategy: "unwind".into(),
        extraction_policy: "source-local/1".into(),
    }
}

fn snapshot() -> Snapshot {
    Snapshot {
        versions: Versions {
            detector: "fn_spacing".into(),
            detector_version: 1,
            identity_version: 1,
            locator_version: 1,
            context_version: 1,
        },
        locator: Locator {
            package: Package {
                identity: "Cargo.toml".into(),
                name: "sample".into(),
                version: "1.0.0".into(),
            },
            source: "src/lib.rs".into(),
            qualified_name: "crate::unseparated".into(),
            declaration_kind: "free-function".into(),
            discriminator: "unique-declaration".into(),
        },
        context: context(),
        policy: Policy { min_statements: 5 },
        measurement: Measurement {
            direct_statements: 5,
        },
    }
}

fn dismissed(snapshot: &Snapshot) -> Vec<Entry> {
    let mut entries = vec![];
    record_dismissal(
        &mut entries,
        &LiveFinding::checked(snapshot).unwrap(),
        "reviewed numeric spacing".into(),
    )
    .unwrap();
    entries
}

fn reason(entries: &[Entry], current: &Snapshot) -> StaleReason {
    let live = LiveFinding::checked(current).unwrap();
    let Lookup::Stale(stale) = ValidatedStore::new(entries).unwrap().lookup(&live) else {
        panic!("expected stale numeric choice");
    };
    assert_eq!(stale.len(), 1);
    assert!(std::ptr::eq(stale[0].entry, &entries[0]));
    stale[0].reason
}

#[test]
fn exact_numeric_choice_roundtrips_without_retaining_source_or_positions() {
    let current = snapshot();
    let bytes = serde_json::to_string(&dismissed(&current)).unwrap();
    for forbidden in [
        "body",
        "digest",
        "hash",
        "report_id",
        "fn_span",
        "matching_policy",
        "function_key",
        "signature",
    ] {
        assert!(!bytes.contains(forbidden), "{forbidden}");
    }
    let entries: Vec<Entry> = serde_json::from_str(&bytes).unwrap();
    let store = ValidatedStore::new(&entries).unwrap();
    let Lookup::HiddenExact(entry) = store.lookup(&LiveFinding::checked(&current).unwrap()) else {
        panic!("expected exact review");
    };
    assert!(std::ptr::eq(entry, &entries[0]));
    assert_eq!(entry.reason, "reviewed numeric spacing");
}

#[test]
fn repeated_positive_detection_with_same_counts_preserves_review() {
    // Both snapshots represent completed positive detections, even if source changed.
    // This model does not infer absence of separating blank lines from the count.
    let before = snapshot();
    let entries = dismissed(&before);
    let after = snapshot();
    assert!(matches!(
        ValidatedStore::new(&entries)
            .unwrap()
            .lookup(&LiveFinding::checked(&after).unwrap()),
        Lookup::HiddenExact(_)
    ));
}

#[test]
fn versions_context_policy_and_statement_measurements_stale_independently() {
    let original = snapshot();
    let entries = dismissed(&original);
    let mut current = original.clone();
    current.versions.detector_version = 2;
    assert_eq!(reason(&entries, &current), StaleReason::Versions);
    current = original.clone();
    current.context.overflow_checks = false;
    assert_eq!(reason(&entries, &current), StaleReason::Context);
    current = original.clone();
    current.policy.min_statements = 4;
    assert_eq!(reason(&entries, &current), StaleReason::Policy);
    current = original;
    current.measurement.direct_statements = 6;
    assert_eq!(reason(&entries, &current), StaleReason::Measurement);
}

#[test]
fn disabled_and_below_threshold_counts_reject_but_threshold_is_valid() {
    let mut current = snapshot();
    current.policy.min_statements = 0;
    assert_eq!(validate_snapshot(&current), Err(Error::InvalidExpression));
    for direct_statements in [0, 4] {
        current = snapshot();
        current.measurement.direct_statements = direct_statements;
        assert_eq!(validate_snapshot(&current), Err(Error::InvalidExpression));
    }
    for direct_statements in [5, 6, usize::MAX] {
        current = snapshot();
        current.measurement.direct_statements = direct_statements;
        assert!(LiveFinding::checked(&current).is_ok());
    }
}

#[test]
fn mixed_locators_and_compilation_targets_do_not_hide_or_stale_spacing_choices() {
    let original = snapshot();
    let mut entries = dismissed(&original);
    let mut sibling = original.clone();
    sibling.context.compilation_target.test = true;
    let live = LiveFinding::checked(&sibling).unwrap();
    assert!(matches!(
        ValidatedStore::new(&entries).unwrap().lookup(&live),
        Lookup::VisibleNew
    ));
    record_dismissal(&mut entries, &live, "test-specific numeric choice".into()).unwrap();
    let store = ValidatedStore::new(&entries).unwrap();
    let Lookup::HiddenExact(found) = store.lookup(&live) else {
        panic!("expected test choice");
    };
    assert!(std::ptr::eq(found, &entries[1]));
    let mut unrelated = original;
    unrelated.locator.source = "src/other.rs".into();
    assert!(matches!(
        store.lookup(&LiveFinding::checked(&unrelated).unwrap()),
        Lookup::VisibleNew
    ));
}

#[test]
fn wrong_detector_versions_locator_and_context_are_rejected() {
    for field in 0..5 {
        let mut current = snapshot();
        match field {
            0 => current.versions.detector = "fn_params".into(),
            1 => current.versions.detector_version = 0,
            2 => current.versions.identity_version = 0,
            3 => current.versions.locator_version = 0,
            _ => current.versions.context_version = 0,
        }
        assert_eq!(validate_snapshot(&current), Err(Error::InvalidVersion));
    }
    let mut current = snapshot();
    current.locator.source = "../outside.rs".into();
    assert_eq!(validate_snapshot(&current), Err(Error::InvalidLocator));
    current = snapshot();
    current.context.pointer_width = 0;
    assert_eq!(validate_snapshot(&current), Err(Error::InvalidContext));
}

#[test]
fn generic_entry_keeps_closed_spacing_record_fields_and_rejects_claims() {
    let entry = dismissed(&snapshot()).pop().unwrap();
    let base = serde_json::to_value(&entry).unwrap();
    for pointer in [
        "",
        "/finding",
        "/finding/policy",
        "/finding/measurement",
        "/finding/versions",
        "/finding/locator",
        "/finding/context",
    ] {
        let mut changed = base.clone();
        changed
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("proof".into(), serde_json::json!(true));
        assert!(
            serde_json::from_value::<Entry>(changed).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn numeric_reviews_retain_reason_duplicate_and_loaded_budget_validation() {
    let current = snapshot();
    let live = LiveFinding::checked(&current).unwrap();
    let mut entries = dismissed(&current);
    assert_eq!(
        record_dismissal(&mut entries, &live, "again".into()),
        Err(Error::DuplicateReview)
    );
    entries[0].reason.clear();
    assert_eq!(validate_entries(&entries), Err(Error::InvalidReason));
    let initial = dismissed(&current).pop().unwrap();
    assert_eq!(
        validate_entries(&vec![initial; exact_rules::MAX_ENTRIES + 1]),
        Err(Error::ExpressionBudget)
    );
}
