use super::*;
use crate::clones::review::{CompilationTarget, Package};

fn snapshot() -> Snapshot {
    Snapshot {
        versions: Versions {
            detector: "side_effect_density".into(),
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
            qualified_name: "crate::effects".into(),
            declaration_kind: "free-function".into(),
            discriminator: "unique-declaration".into(),
        },
        context: context(),
        policy: Policy::from_config(&SideEffectsConfig {
            min_statements: 0,
            min_lines: 0,
            transitive_depth: 2,
            ..SideEffectsConfig::default()
        }),
        measurement: Measurement {
            statements: 3,
            code_lines: 10,
            event_counts: BTreeMap::from([(EffectKind::Mutation, 2), (EffectKind::Time, 1)]),
            actual: 1000,
        },
    }
}

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

fn dismissed(current: &Snapshot) -> Vec<Entry> {
    let mut entries = vec![];
    record_dismissal(
        &mut entries,
        &LiveFinding::checked(current).unwrap(),
        "reviewed static density".into(),
    )
    .unwrap();
    entries
}

fn stale(original: &Snapshot, current: &Snapshot) -> StaleReason {
    let entries = dismissed(original);
    let live = LiveFinding::checked(current).unwrap();
    let Lookup::Stale(entries) = ValidatedStore::new(&entries).unwrap().lookup(&live) else {
        panic!("expected stale density choice");
    };
    entries[0].reason
}

#[test]
fn exact_numeric_density_roundtrips_and_borrows_the_saved_entry() {
    let current = snapshot();
    let bytes = serde_json::to_string(&dismissed(&current)).unwrap();
    for forbidden in ["digest", "fn_span", "graph", "source_text", "proof"] {
        assert!(!bytes.contains(forbidden));
    }
    let entries: Vec<Entry> = serde_json::from_str(&bytes).unwrap();
    let live = LiveFinding::checked(&current).unwrap();
    let Lookup::HiddenExact(found) = ValidatedStore::new(&entries).unwrap().lookup(&live) else {
        panic!("expected exact numeric choice");
    };
    assert!(std::ptr::eq(found, &entries[0]));
}

#[test]
fn unchanged_ratio_with_changed_event_distribution_remains_visible() {
    let original = snapshot();
    let mut current = original.clone();
    current.measurement.event_counts = BTreeMap::from([(EffectKind::Time, 3)]);
    assert_eq!(stale(&original, &current), StaleReason::Measurement);
    current = original.clone();
    current.measurement.code_lines += 1;
    assert_eq!(stale(&original, &current), StaleReason::Measurement);
    current = original.clone();
    current.measurement.statements = 2;
    current.measurement.actual = 1500;
    assert_eq!(stale(&original, &current), StaleReason::Measurement);
}

#[test]
fn all_numeric_policy_inputs_and_metric_versions_stale_exact_choices() {
    let original = snapshot();
    for field in 0..11 {
        let mut current = original.clone();
        let policy = &mut current.policy;
        match field {
            0 => policy.event_metric.push('2'),
            1 => policy.statement_metric.push('2'),
            2 => policy.code_lines_metric.push('2'),
            3 => policy.classification_metric.push('2'),
            4 => policy.config.max_density_milli -= 1,
            5 => policy.config.min_statements = 1,
            6 => policy.config.min_lines = 1,
            7 => policy.config.weights.panic = 2,
            8 => policy.config.weights.unknown = 0,
            9 => policy.config.paths.io.push("custom::".into()),
            _ => policy.config.paths.time.clear(),
        }
        assert_eq!(stale(&original, &current), StaleReason::Policy);
    }
}

#[test]
fn identity_and_build_changes_use_existing_exact_review_boundaries() {
    let original = snapshot();
    let mut current = original.clone();
    current.versions.detector_version += 1;
    assert_eq!(stale(&original, &current), StaleReason::Versions);
    current = original.clone();
    current.context.debug_assertions = false;
    assert_eq!(stale(&original, &current), StaleReason::Context);
    let entries = dismissed(&original);
    for field in 0..2 {
        current = original.clone();
        if field == 0 {
            current.locator.source = "src/other.rs".into();
        } else {
            current.context.compilation_target.test = true;
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
fn absent_unknown_zero_overflow_exempt_and_nonfindings_cannot_be_saved() {
    for field in 0..10 {
        let mut current = snapshot();
        match field {
            0 => current.measurement.event_counts.clear(),
            1 => {
                current
                    .measurement
                    .event_counts
                    .insert(EffectKind::Unknown, 1);
            }
            2 => {
                current
                    .measurement
                    .event_counts
                    .insert(EffectKind::Panic, 0);
            }
            3 => {
                current
                    .measurement
                    .event_counts
                    .insert(EffectKind::Mutation, usize::MAX);
            }
            4 => current.policy.config.min_statements = 4,
            5 => current.policy.config.min_lines = 11,
            6 => current.policy.config.max_density_milli = current.measurement.actual,
            7 => current.measurement.actual += 1,
            8 => current.policy.config.transitive_depth = 3,
            _ => current.policy.event_metric.clear(),
        }
        assert!(LiveFinding::checked(&current).is_err(), "{field}");
    }
}

#[test]
fn zero_statement_denominator_and_checked_floor_are_exact() {
    let mut current = snapshot();
    current.measurement.statements = 0;
    current.measurement.actual = 3000;
    assert!(LiveFinding::checked(&current).is_ok());
    current.measurement.statements = 4;
    current.measurement.actual = 750;
    assert!(LiveFinding::checked(&current).is_ok());
}

#[test]
fn loaded_density_records_reject_unknown_fields_duplicate_choices_and_empty_reasons() {
    let current = snapshot();
    let entry = dismissed(&current).pop().unwrap();
    let base = serde_json::to_value(&entry).unwrap();
    for pointer in [
        "",
        "/finding",
        "/finding/policy",
        "/finding/policy/config",
        "/finding/measurement",
    ] {
        let mut changed = base.clone();
        changed
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("complete".into(), serde_json::json!(true));
        assert!(serde_json::from_value::<Entry>(changed).is_err());
    }
    assert_eq!(
        validate_entries(&[entry.clone(), entry.clone()]),
        Err(Error::DuplicateReview)
    );
    let mut changed = entry;
    changed.reason.clear();
    assert_eq!(validate_entries(&[changed]), Err(Error::InvalidReason));
}

#[test]
fn density_envelope_roundtrips_only_with_its_supported_schema() {
    use crate::clones::review::ReviewStore;

    let store = ReviewStore {
        schema_version: 7,
        density_entries: dismissed(&snapshot()),
        ..ReviewStore::default()
    };
    let bytes = store.to_json().unwrap();
    assert_eq!(ReviewStore::from_json(&bytes).unwrap(), store);
    let mut wire: serde_json::Value = serde_json::from_str(&bytes).unwrap();
    for supported in 7..=10 {
        wire["schema_version"] = serde_json::json!(supported);
        assert_eq!(
            ReviewStore::from_json(&wire.to_string())
                .unwrap()
                .density_entries,
            store.density_entries
        );
    }
    for unsupported in [6, 11] {
        wire["schema_version"] = serde_json::json!(unsupported);
        assert!(ReviewStore::from_json(&wire.to_string()).is_err());
    }
}

#[test]
fn legacy_empty_review_envelopes_remain_readable_without_density_entries() {
    use crate::clones::review::ReviewStore;

    for schema_version in 1..=6 {
        let wire = serde_json::json!({"schema_version": schema_version, "entries": []});
        let store = ReviewStore::from_json(&wire.to_string()).unwrap();
        assert_eq!(store.schema_version, schema_version);
        assert!(store.density_entries.is_empty());
        assert_eq!(
            ReviewStore::from_json(&store.to_json().unwrap()).unwrap(),
            store
        );
    }
}
