use super::*;
use crate::clones::review::{CompilationTarget, Package};

fn snapshot() -> Snapshot {
    Snapshot {
        versions: Versions {
            detector: "fn_params".into(),
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
            qualified_name: "crate::many".into(),
            declaration_kind: "free-function".into(),
            discriminator: "unique-declaration".into(),
        },
        context: local::BuildContext {
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
        },
        policy: Policy {
            max_params: 4,
            count_self: false,
        },
        signature: Signature {
            parameters: 5,
            has_self: false,
            actual: 5,
        },
    }
}

fn dismissed(snapshot: &Snapshot) -> Vec<Entry> {
    let mut entries = vec![];
    record_dismissal(
        &mut entries,
        &LiveFinding::checked(snapshot).unwrap(),
        "reviewed".into(),
    )
    .unwrap();
    entries
}

fn reason(entries: &[Entry], current: &Snapshot) -> StaleReason {
    let live = LiveFinding::checked(current).unwrap();
    let Lookup::Stale(stale) = ValidatedStore::new(entries).unwrap().lookup(&live) else {
        panic!("expected stale finding");
    };
    assert_eq!(stale.len(), 1);
    assert!(std::ptr::eq(stale[0].entry, &entries[0]));
    stale[0].reason
}

#[test]
fn exact_roundtrip_borrows_stored_choice_and_contains_no_body_or_run_identity() {
    let current = snapshot();
    let entries = dismissed(&current);
    let bytes = serde_json::to_string(&entries).unwrap();
    for forbidden in [
        "body",
        "digest",
        "matching_policy",
        "function_key",
        "fn_span",
    ] {
        assert!(!bytes.contains(forbidden), "{forbidden}");
    }
    let loaded: Vec<Entry> = serde_json::from_str(&bytes).unwrap();
    let store = ValidatedStore::new(&loaded).unwrap();
    for _ in 0..2 {
        let Lookup::HiddenExact(entry) = store.lookup(&LiveFinding::checked(&current).unwrap())
        else {
            panic!("expected exact retained choice");
        };
        assert!(std::ptr::eq(entry, &loaded[0]));
        assert_eq!(entry.reason, "reviewed");
    }
}

#[test]
fn version_context_policy_and_signature_changes_have_distinct_reasons() {
    let original = snapshot();
    let entries = dismissed(&original);
    let mut current = original.clone();
    current.versions.detector_version = 2;
    assert_eq!(reason(&entries, &current), StaleReason::Versions);
    current = original.clone();
    current.context.debug_assertions = false;
    assert_eq!(reason(&entries, &current), StaleReason::Context);
    current = original.clone();
    current.policy.max_params = 3;
    assert_eq!(reason(&entries, &current), StaleReason::Policy);
    current = original;
    current.signature.parameters = 6;
    current.signature.actual = 6;
    assert_eq!(reason(&entries, &current), StaleReason::Signature);
}

#[test]
fn count_self_policy_changes_stale_even_when_measured_actual_is_unchanged() {
    let original = snapshot();
    let entries = dismissed(&original);
    let mut current = original;
    current.policy.count_self = true;
    assert_eq!(reason(&entries, &current), StaleReason::Policy);
}

#[test]
fn receiver_counts_are_checked_with_and_without_receiver_policy() {
    for count_self in [false, true] {
        for has_self in [false, true] {
            let mut current = snapshot();
            current.policy.count_self = count_self;
            current.signature.has_self = has_self;
            current.signature.actual = 5 + usize::from(count_self && has_self);
            assert!(validate_snapshot(&current).is_ok());
            current.signature.actual += 1;
            assert_eq!(validate_snapshot(&current), Err(Error::InvalidSignature));
        }
    }
}

#[test]
fn inconsistent_clean_or_overflowing_signatures_cannot_mint_live_findings() {
    let mut current = snapshot();
    current.signature.actual = 4;
    assert!(LiveFinding::checked(&current).is_err());
    current = snapshot();
    current.policy.max_params = 5;
    assert!(LiveFinding::checked(&current).is_err());
    current = snapshot();
    current.policy.count_self = true;
    current.signature.has_self = true;
    current.signature.parameters = usize::MAX;
    current.signature.actual = usize::MAX;
    assert!(LiveFinding::checked(&current).is_err());
}

#[test]
fn unrelated_locators_never_hide_or_stale_an_existing_choice() {
    let original = snapshot();
    let entries = dismissed(&original);
    for attack in 0..4 {
        let mut current = original.clone();
        match attack {
            0 => current.locator.qualified_name = "crate::other::many".into(),
            1 => current.locator.source = "src/other.rs".into(),
            2 => current.locator.package.identity = "nested/Cargo.toml".into(),
            _ => current.locator.declaration_kind = "method".into(),
        }
        assert!(matches!(
            ValidatedStore::new(&entries)
                .unwrap()
                .lookup(&LiveFinding::checked(&current).unwrap()),
            Lookup::VisibleNew
        ));
    }
}

#[test]
fn compilation_targets_are_independent_and_exact_sibling_entries_can_coexist() {
    let original = snapshot();
    let mut entries = dismissed(&original);
    let mut sibling = original.clone();
    sibling.context.compilation_target.test = true;
    sibling.context.cfgs = vec!["test".into(), "unix".into()];
    let live = LiveFinding::checked(&sibling).unwrap();
    assert!(matches!(
        ValidatedStore::new(&entries).unwrap().lookup(&live),
        Lookup::VisibleNew
    ));
    record_dismissal(&mut entries, &live, "test adapter".into()).unwrap();
    assert_eq!(entries.len(), 2);
    let Lookup::HiddenExact(found) = ValidatedStore::new(&entries).unwrap().lookup(&live) else {
        panic!("expected target-specific review");
    };
    assert_eq!(found.reason, "test adapter");
    let mut changed = original;
    changed.context.compilation_target.name = "different-target".into();
    assert!(matches!(
        ValidatedStore::new(&entries)
            .unwrap()
            .lookup(&LiveFinding::checked(&changed).unwrap()),
        Lookup::VisibleNew
    ));
}

#[test]
fn invalid_versions_locator_and_build_context_are_rejected() {
    for version in 0..5 {
        let mut current = snapshot();
        match version {
            0 => current.versions.detector = "fn_length".into(),
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
    current = snapshot();
    current.context.cfgs = vec!["unix".into(), "unix".into()];
    assert_eq!(validate_snapshot(&current), Err(Error::InvalidContext));
}

#[test]
fn duplicate_findings_and_empty_reasons_reject_without_modifying_entries() {
    let current = snapshot();
    let live = LiveFinding::checked(&current).unwrap();
    let mut entries = dismissed(&current);
    assert_eq!(
        record_dismissal(&mut entries, &live, "again".into()),
        Err(Error::DuplicateReview)
    );
    assert_eq!(entries.len(), 1);
    let mut duplicated = entries.clone();
    duplicated[0].reason = "another reason".into();
    duplicated.extend(entries.clone());
    assert_eq!(validate_entries(&duplicated), Err(Error::DuplicateReview));
    assert_eq!(
        record_dismissal(&mut entries, &live, " \n\t".into()),
        Err(Error::InvalidReason)
    );
    entries[0].reason.clear();
    assert_eq!(validate_entries(&entries), Err(Error::InvalidReason));
}

#[test]
fn every_persisted_record_rejects_unknown_fields() {
    let entry = dismissed(&snapshot()).pop().unwrap();
    let base = serde_json::to_value(&entry).unwrap();
    for pointer in [
        "",
        "/finding",
        "/finding/policy",
        "/finding/signature",
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
fn entry_budget_applies_to_loaded_and_new_choices() {
    let initial = dismissed(&snapshot()).pop().unwrap();
    assert_eq!(
        validate_entries(&vec![initial.clone(); MAX_ENTRIES + 1]),
        Err(Error::ExpressionBudget)
    );
    let mut entries: Vec<_> = (0..MAX_ENTRIES)
        .map(|index| {
            let mut entry = initial.clone();
            entry.finding.locator.qualified_name = format!("crate::many_{index}");
            entry
        })
        .collect();
    assert!(validate_entries(&entries).is_ok());
    let current = snapshot();
    assert_eq!(
        record_dismissal(
            &mut entries,
            &LiveFinding::checked(&current).unwrap(),
            "new".into()
        ),
        Err(Error::ExpressionBudget)
    );
    assert_eq!(entries.len(), MAX_ENTRIES);
}

#[test]
fn mixed_locators_and_target_contexts_preserve_exact_and_relevant_stale_choices() {
    let original = snapshot();
    let mut entries = dismissed(&original);
    let mut unrelated = original.clone();
    unrelated.locator.qualified_name = "crate::other".into();
    entries.extend(dismissed(&unrelated));
    let mut sibling = original.clone();
    sibling.context.compilation_target.test = true;
    entries.extend(dismissed(&sibling));
    let store = ValidatedStore::new(&entries).unwrap();
    let Lookup::HiddenExact(found) = store.lookup(&LiveFinding::checked(&sibling).unwrap()) else {
        panic!("expected exact sibling choice");
    };
    assert!(std::ptr::eq(found, &entries[2]));
    let mut changed = original;
    changed.signature.parameters = 6;
    changed.signature.actual = 6;
    let Lookup::Stale(stale) = store.lookup(&LiveFinding::checked(&changed).unwrap()) else {
        panic!("expected signature change");
    };
    assert_eq!(stale.len(), 1);
    assert!(std::ptr::eq(stale[0].entry, &entries[0]));
    assert_eq!(stale[0].reason, StaleReason::Signature);
}
