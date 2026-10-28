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
            detector: "mutation_per_target".into(),
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
            qualified_name: "crate::mutating".into(),
            declaration_kind: "free-function".into(),
            discriminator: "unique-declaration".into(),
        },
        context: context(),
        policy: Policy::MutationPerTarget {
            max_mutations_per_target: 2,
        },
        measurement: Measurement {
            sites_per_root: vec![1, 3],
        },
    }
}

fn dismissed(snapshot: &Snapshot) -> Vec<Entry> {
    let mut entries = vec![];
    record_dismissal(
        &mut entries,
        &LiveFinding::checked(snapshot).unwrap(),
        "reviewed numeric mutations".into(),
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
    assert_eq!(entry.reason, "reviewed numeric mutations");
}

#[test]
fn versions_context_policy_and_count_measurements_stale_independently() {
    let original = snapshot();
    let entries = dismissed(&original);
    let mut current = original.clone();
    current.versions.detector_version = 2;
    assert_eq!(reason(&entries, &current), StaleReason::Versions);
    current = original.clone();
    current.context.overflow_checks = false;
    assert_eq!(reason(&entries, &current), StaleReason::Context);
    current = original.clone();
    current.policy = Policy::MutationPerTarget {
        max_mutations_per_target: 1,
    };
    assert_eq!(reason(&entries, &current), StaleReason::Policy);
    current = original;
    current.measurement.sites_per_root = vec![2, 3];
    assert_eq!(reason(&entries, &current), StaleReason::Measurement);
}

#[test]
fn mixed_locators_and_compilation_targets_do_not_hide_or_stale_mutation_choices() {
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
fn generic_entry_keeps_closed_mutation_record_fields_and_rejects_claims() {
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

#[test]
fn both_rule_families_require_a_strict_numeric_breach() {
    for policy in [
        Policy::MutationPerTarget {
            max_mutations_per_target: 2,
        },
        Policy::MutatedTargets {
            max_mutated_targets: 1,
        },
    ] {
        let mut current = snapshot();
        current.versions.detector = policy.rule_id().into();
        current.policy = policy;
        let actual = current.measurement.actual(&current.policy);
        assert_eq!(current.policy.limit() + 1, actual);
        assert!(LiveFinding::checked(&current).is_ok());
        current.policy = match current.policy {
            Policy::MutationPerTarget { .. } => Policy::MutationPerTarget {
                max_mutations_per_target: actual,
            },
            Policy::MutatedTargets { .. } => Policy::MutatedTargets {
                max_mutated_targets: actual,
            },
        };
        assert_eq!(validate_snapshot(&current), Err(Error::InvalidExpression));
    }
}

#[test]
fn canonical_distribution_rejects_zero_unsorted_overflow_and_overbudget_counts() {
    for counts in [
        vec![],
        vec![0, 3],
        vec![3, 1],
        vec![4097],
        vec![1, 4096],
        vec![usize::MAX, usize::MAX],
        vec![1; 4097],
    ] {
        let mut current = snapshot();
        current.measurement.sites_per_root = counts;
        assert_eq!(validate_snapshot(&current), Err(Error::InvalidExpression));
    }
    let mut current = snapshot();
    current.measurement.sites_per_root = vec![4096];
    assert!(validate_snapshot(&current).is_ok());
    current.policy = Policy::MutatedTargets {
        max_mutated_targets: 4095,
    };
    current.versions.detector = current.policy.rule_id().into();
    current.measurement.sites_per_root = vec![1; 4096];
    assert!(validate_snapshot(&current).is_ok());
}

#[test]
fn changed_distribution_stales_even_when_maximum_root_and_site_counts_match() {
    let mut original = snapshot();
    original.measurement.sites_per_root = vec![1, 3, 4];
    let entries = dismissed(&original);
    let mut current = original.clone();
    current.measurement.sites_per_root = vec![2, 2, 4];
    assert_eq!(
        original.measurement.actual(&original.policy),
        current.measurement.actual(&current.policy)
    );
    assert_eq!(
        original.measurement.sites_per_root.len(),
        current.measurement.sites_per_root.len()
    );
    assert_eq!(
        original.measurement.sites_per_root.iter().sum::<usize>(),
        current.measurement.sites_per_root.iter().sum::<usize>()
    );
    assert_eq!(reason(&entries, &current), StaleReason::Measurement);
}

#[test]
fn numeric_choices_keep_equal_distributions_without_retaining_authored_work() {
    let current = snapshot();
    let entries = dismissed(&current);
    let bytes = serde_json::to_string(&entries).unwrap();
    for forbidden in [
        "hir",
        "native",
        "scope_span",
        "declaration_span",
        "operation",
        "rhs",
        "source_digest",
        "coverage",
        "availability",
        "execution",
    ] {
        assert!(!bytes.contains(forbidden), "{forbidden}");
    }
    let loaded: Vec<Entry> = serde_json::from_str(&bytes).unwrap();
    let live = LiveFinding::checked(&current).unwrap();
    assert!(matches!(
        ValidatedStore::new(&loaded).unwrap().lookup(&live),
        Lookup::HiddenExact(_)
    ));
}

#[test]
fn policy_tag_and_version_detector_cannot_disagree_or_accept_other_rule_fields() {
    let mut current = snapshot();
    current.versions.detector = "mutated_targets".into();
    assert_eq!(validate_snapshot(&current), Err(Error::InvalidVersion));
    assert!(
        serde_json::from_value::<Policy>(serde_json::json!({
            "kind": "mutation_per_target", "max_mutations_per_target": 2,
            "max_mutated_targets": 1
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<Policy>(serde_json::json!({
            "kind": "mutated_targets", "max_mutations_per_target": 2
        }))
        .is_err()
    );
}

#[test]
fn rule_families_partition_stale_lookup_without_skipping_loaded_validation() {
    let original = snapshot();
    let mut entries = dismissed(&original);
    let mut targets = original.clone();
    targets.policy = Policy::MutatedTargets {
        max_mutated_targets: 1,
    };
    targets.versions.detector = targets.policy.rule_id().into();
    let targets_live = LiveFinding::checked(&targets).unwrap();
    let target_store = ValidatedStore::new_filtered(&entries, |entry| {
        entry.finding.policy.rule_id() == targets.policy.rule_id()
    })
    .unwrap();
    assert!(matches!(
        target_store.lookup(&targets_live),
        Lookup::VisibleNew
    ));
    record_dismissal(&mut entries, &targets_live, "reviewed roots".into()).unwrap();
    let store = ValidatedStore::new_filtered(&entries, |entry| {
        entry.finding.policy.rule_id() == targets.policy.rule_id()
    })
    .unwrap();
    let Lookup::HiddenExact(found) = store.lookup(&targets_live) else {
        panic!("expected exact root-count choice");
    };
    assert!(std::ptr::eq(found, &entries[1]));
    let mut changed = targets.clone();
    changed.measurement.sites_per_root = vec![2, 3];
    let Lookup::Stale(stale) = store.lookup(&LiveFinding::checked(&changed).unwrap()) else {
        panic!("expected only same-family stale root-count choice");
    };
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].reason, StaleReason::Measurement);
    entries[0].reason.clear();
    assert!(matches!(
        ValidatedStore::new_filtered(&entries, |entry| {
            entry.finding.policy.rule_id() == targets.policy.rule_id()
        }),
        Err(Error::InvalidReason)
    ));
}
