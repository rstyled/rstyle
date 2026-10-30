use super::*;
use crate::clones::review::{CompilationTarget, Locator, Package, Versions, density_rules, local};
use rstyle_core::{config::SideEffectsConfig, facts::EffectKind};
use std::collections::BTreeMap;

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
        policy: density_rules::Policy::from_config(&SideEffectsConfig {
            min_statements: 0,
            min_lines: 0,
            transitive_depth: 2,
            ..SideEffectsConfig::default()
        }),
        measurement: density_rules::Measurement {
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

#[test]
fn empty_and_numeric_baselines_roundtrip_without_analysis_claims() {
    for entries in [vec![], vec![snapshot()]] {
        let baseline = make_baseline(entries, snapshot().policy).unwrap();
        let bytes = serde_json::to_string(&baseline).unwrap();
        for forbidden in ["digest", "fn_span", "graph", "source_text", "proof"] {
            assert!(!bytes.contains(forbidden));
        }
        let loaded: Baseline = serde_json::from_str(&bytes).unwrap();
        loaded.validate().unwrap();
        assert_eq!(loaded, baseline);
    }
}

#[test]
fn unsupported_schemas_and_unknown_fields_fail_closed() {
    let baseline = make_baseline(vec![snapshot()], snapshot().policy).unwrap();
    let base = serde_json::to_value(&baseline).unwrap();
    for pointer in [
        "",
        "/policy",
        "/versions",
        "/entries/0",
        "/entries/0/measurement",
    ] {
        let mut wire = base.clone();
        wire.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("complete".into(), serde_json::json!(true));
        assert!(serde_json::from_value::<Baseline>(wire).is_err());
    }
    for schema in ["", "rstyle.density-baseline/0", "rstyle.density-baseline/2"] {
        let mut wire = base.clone();
        wire["schema"] = serde_json::json!(schema);
        let loaded: Baseline = serde_json::from_value(wire).unwrap();
        assert!(loaded.validate().is_err());
    }
}

#[test]
fn every_snapshot_requires_valid_numeric_and_original_identity_values() {
    for field in 0..4 {
        let mut invalid = snapshot();
        match field {
            0 => invalid.measurement.actual += 1,
            1 => invalid.policy.event_metric.clear(),
            2 => invalid.locator.source.clear(),
            _ => invalid.context.compiler_commit.clear(),
        }
        assert!(
            make_baseline(vec![invalid], snapshot().policy).is_err(),
            "{field}"
        );
    }
}

#[test]
fn same_original_identity_is_ambiguous_even_with_changed_numeric_or_build_context() {
    let original = snapshot();
    for field in 0..4 {
        let mut duplicate = original.clone();
        match field {
            0 => {}
            1 => duplicate.context.debug_assertions = false,
            2 => duplicate.measurement.code_lines += 1,
            _ => duplicate.policy.config.max_density_milli -= 1,
        }
        assert!(
            make_baseline(vec![original.clone(), duplicate], snapshot().policy).is_err(),
            "{field}"
        );
    }
}

#[test]
fn distinct_original_declarations_and_compilation_targets_can_coexist() {
    let original = snapshot();
    let mut declaration = original.clone();
    declaration.locator.qualified_name = "crate::other".into();
    let mut target = original.clone();
    target.context.compilation_target.test = true;
    assert!(make_baseline(vec![original, declaration, target], snapshot().policy).is_ok());
}

#[test]
fn entry_limit_accepts_4096_and_rejects_one_more() {
    let entries: Vec<_> = (0..=MAX_ENTRIES)
        .map(|index| {
            let mut entry = snapshot();
            entry.locator.qualified_name = format!("crate::function_{index}");
            entry
        })
        .collect();
    assert!(make_baseline(entries[..MAX_ENTRIES].to_vec(), snapshot().policy).is_ok());
    assert!(make_baseline(entries, snapshot().policy).is_err());
}

#[test]
fn empty_inventory_still_requires_valid_policy_and_supported_versions() {
    let mut baseline = make_baseline(vec![], snapshot().policy).unwrap();
    baseline.policy.event_metric.clear();
    assert!(baseline.validate().is_err());
    baseline.policy = snapshot().policy;
    baseline.policy.config.transitive_depth = 3;
    assert!(baseline.validate().is_err());
    for field in 0..5 {
        let mut baseline = make_baseline(vec![], snapshot().policy).unwrap();
        match field {
            0 => baseline.versions.detector = "other".into(),
            1 => baseline.versions.detector_version += 1,
            2 => baseline.versions.identity_version += 1,
            3 => baseline.versions.locator_version += 1,
            _ => baseline.versions.context_version += 1,
        }
        assert!(baseline.validate().is_err(), "{field}");
    }
}

#[test]
fn entries_require_the_same_policy_and_versions_as_the_header() {
    for field in 0..2 {
        let mut changed = snapshot();
        if field == 0 {
            changed.policy.config.max_density_milli -= 1;
        } else {
            changed.versions.detector_version += 1;
        }
        assert!(density_rules::validate_snapshot(&changed).is_ok());
        assert!(make_baseline(vec![changed], snapshot().policy).is_err());
    }
}

fn make_baseline(
    entries: Vec<Snapshot>,
    policy: density_rules::Policy,
) -> Result<Baseline, String> {
    let contexts = entries.iter().map(|entry| entry.context.clone()).collect();
    Baseline::new(entries, policy, contexts, vec![])
}

#[test]
fn context_header_is_validated_even_without_findings() {
    let mut invalid = context();
    invalid.compiler_commit.clear();
    assert!(Baseline::new(vec![], snapshot().policy, vec![invalid], vec![]).is_err());
    let first = context();
    let mut second = first.clone();
    second.debug_assertions = false;
    let baseline = Baseline::new(
        vec![],
        snapshot().policy,
        vec![first.clone(), second.clone(), first],
        vec![],
    )
    .unwrap();
    assert_eq!(baseline.contexts.len(), 2);
    assert!(baseline.contexts[0] < baseline.contexts[1]);
    let mut malformed = baseline;
    malformed.contexts.reverse();
    assert!(malformed.validate().is_err());
    malformed.contexts = vec![second.clone(), second];
    assert!(malformed.validate().is_err());
}

#[test]
fn findings_require_a_matching_retained_context_and_roster_is_bounded() {
    assert!(Baseline::new(vec![snapshot()], snapshot().policy, vec![], vec![]).is_err());
    let mut baseline = make_baseline(vec![], snapshot().policy).unwrap();
    baseline.contexts = (0..=MAX_ENTRIES)
        .map(|index| {
            let mut entry = context();
            entry.compiler_commit = format!("compiler_{index:05}");
            entry
        })
        .collect();
    assert!(baseline.validate().is_err());
    baseline.contexts.pop();
    assert!(baseline.validate().is_ok());
}

#[test]
fn exclusion_header_requires_valid_globs_and_a_bounded_roster() {
    assert!(Baseline::new(vec![], snapshot().policy, vec![], vec!["[".into()]).is_err());
    let exclude = vec!["target/**".into(); MAX_ENTRIES];
    let mut baseline = Baseline::new(vec![], snapshot().policy, vec![], exclude).unwrap();
    baseline.exclude.push("src/generated/**".into());
    assert!(baseline.validate().is_err());
}
