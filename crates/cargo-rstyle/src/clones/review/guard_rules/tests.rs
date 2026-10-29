use super::*;
use crate::clones::review::exact_rules::{LiveFinding, Lookup, ReviewSnapshot, ValidatedStore};
use crate::clones::review::{CompilationTarget, Package};

fn snapshot() -> Snapshot {
    Snapshot {
        versions: Versions {
            detector: "guard_clause".into(),
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
        region: Region::FunctionTail,
        guard_source: "if x { 0 } else { let y = x; y }".into(),
        closure_source: None,
        fallback_branch: Branch::Then,
        continuing_statements: 2,
    }
}

fn retained(snapshot: &Snapshot) -> Vec<Entry> {
    let mut entries = vec![];
    exact_rules::record_dismissal(
        &mut entries,
        &LiveFinding::checked(snapshot).unwrap(),
        "reviewed".into(),
    )
    .unwrap();
    entries
}

#[test]
fn exact_region_roundtrip_and_changed_source_are_distinct() {
    let original = snapshot();
    let bytes = serde_json::to_string(&retained(&original)).unwrap();
    let entries: Vec<Entry> = serde_json::from_str(&bytes).unwrap();
    let store = ValidatedStore::new(&entries).unwrap();
    assert!(matches!(
        store.lookup(&LiveFinding::checked(&original).unwrap()),
        Lookup::HiddenExact(_)
    ));
    let mut changed = original.clone();
    changed.guard_source = changed.guard_source.replace("0", "1");
    let Lookup::Stale(found) = store.lookup(&LiveFinding::checked(&changed).unwrap()) else {
        panic!("expected stale guard");
    };
    assert_eq!(found[0].reason, exact_rules::StaleReason::Measurement);
}

#[test]
fn closure_slots_and_enclosing_sources_preserve_independent_choices() {
    let mut original = snapshot();
    original.region = Region::Closure { ordinal: 0 };
    original.closure_source = Some(format!("|x| {{ {} }}", original.guard_source));
    let entries = retained(&original);
    let store = ValidatedStore::new(&entries).unwrap();
    let mut sibling = original.clone();
    sibling.region = Region::Closure { ordinal: 1 };
    assert!(matches!(
        store.lookup(&LiveFinding::checked(&sibling).unwrap()),
        Lookup::VisibleNew
    ));
    let mut changed = original.clone();
    changed.closure_source = Some(format!("move |x| {{ {} }}", original.guard_source));
    assert!(matches!(
        store.lookup(&LiveFinding::checked(&changed).unwrap()),
        Lookup::Stale(_)
    ));
}

#[test]
fn malformed_or_oversized_regions_cannot_mint_live_reviews() {
    let mut invalid = snapshot();
    invalid.closure_source = Some(invalid.guard_source.clone());
    assert!(invalid.validate().is_err());
    invalid.region = Region::Closure { ordinal: 0 };
    invalid.closure_source = Some("unrelated".into());
    assert!(invalid.validate().is_err());
    invalid.closure_source = Some(invalid.guard_source.clone());
    invalid.continuing_statements = 1;
    assert!(invalid.validate().is_err());
    invalid.continuing_statements = 2;
    invalid.guard_source = "a".repeat(MAX_SOURCE + 1);
    assert!(invalid.validate().is_err());
}

#[test]
fn versions_context_and_shape_changes_cannot_hide_old_choices() {
    let original = snapshot();
    let entries = retained(&original);
    let store = ValidatedStore::new(&entries).unwrap();
    let mut variants = [original.clone(), original.clone(), original.clone()];
    variants[0].versions.detector_version += 1;
    variants[1].context.overflow_checks = false;
    variants[2].fallback_branch = Branch::Else;
    for (current, expected) in variants.iter().zip([
        exact_rules::StaleReason::Versions,
        exact_rules::StaleReason::Context,
        exact_rules::StaleReason::Measurement,
    ]) {
        let Lookup::Stale(found) = store.lookup(&LiveFinding::checked(current).unwrap()) else {
            panic!("expected changed guard review");
        };
        assert_eq!(found[0].reason, expected);
    }
}
