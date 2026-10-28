use super::*;
use crate::clones::review::{CompilationTarget, Package};

fn versions(detector: &str) -> Versions {
    Versions {
        detector: detector.into(),
        detector_version: 1,
        identity_version: 1,
        locator_version: 1,
        context_version: 1,
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

fn locator() -> Locator {
    Locator {
        package: Package {
            identity: "Cargo.toml".into(),
            name: "sample".into(),
            version: "1.0.0".into(),
        },
        source: "src/lib.rs".into(),
        qualified_name: "crate::sample".into(),
        declaration_kind: "free-function".into(),
        discriminator: "unique-declaration".into(),
    }
}

fn policy() -> Policy {
    Policy {
        parameters: source_rules::Policy {
            max_params: 2,
            count_self: true,
        },
        lengths: length_rules::Policy { max_lines: 10 },
        spacings: Some(spacing_rules::Policy { min_statements: 3 }),
    }
}

fn findings() -> Vec<Finding> {
    vec![
        Finding::Parameter(source_rules::Snapshot {
            versions: versions("fn_params"),
            locator: locator(),
            context: context(),
            policy: policy().parameters,
            signature: source_rules::Signature {
                parameters: 2,
                has_self: true,
                actual: 3,
            },
        }),
        Finding::Length(length_rules::Snapshot {
            versions: versions("fn_length"),
            locator: locator(),
            context: context(),
            policy: policy().lengths,
            measurement: length_rules::Measurement { code_lines: 11 },
        }),
        Finding::Spacing(spacing_rules::Snapshot {
            versions: versions("fn_spacing"),
            locator: locator(),
            context: context(),
            policy: policy().spacings.unwrap(),
            measurement: spacing_rules::Measurement {
                direct_statements: 3,
            },
        }),
    ]
}

fn baseline(entries: Vec<Finding>) -> Baseline {
    Baseline::new(entries, policy(), vec![context()], vec![]).unwrap()
}

#[test]
fn empty_and_three_rule_inventories_preserve_complete_headers() {
    for entries in [vec![], findings()] {
        let original = baseline(entries);
        let bytes = serde_json::to_string(&original).unwrap();
        let loaded: Baseline = serde_json::from_str(&bytes).unwrap();
        loaded.validate().unwrap();
        assert_eq!(loaded, original);
        assert_eq!(loaded.policy, policy());
        assert_eq!(loaded.versions.parameters, versions("fn_params"));
    }
}

#[test]
fn unknown_fields_and_unknown_rule_tags_are_rejected() {
    let base = serde_json::to_value(baseline(findings())).unwrap();
    for pointer in [
        "",
        "/policy",
        "/policy/parameters",
        "/versions",
        "/versions/lengths",
        "/entries/0",
        "/entries/0/finding",
        "/entries/0/finding/signature",
    ] {
        let mut wire = base.clone();
        wire.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("proof".into(), serde_json::json!(true));
        assert!(
            serde_json::from_value::<Baseline>(wire).is_err(),
            "{pointer}"
        );
    }
    let mut wire = base;
    wire["entries"][0]["rule"] = serde_json::json!("Unknown");
    assert!(serde_json::from_value::<Baseline>(wire).is_err());
}

#[test]
fn empty_inventory_validates_versions_schema_and_spacing_policy() {
    for pointer in [
        "/versions/parameters/detector_version",
        "/versions/lengths/identity_version",
        "/versions/spacings/context_version",
    ] {
        let mut wire = serde_json::to_value(baseline(vec![])).unwrap();
        *wire.pointer_mut(pointer).unwrap() = serde_json::json!(2);
        let loaded: Baseline = serde_json::from_value(wire).unwrap();
        assert!(loaded.validate().is_err());
    }
    let mut invalid = baseline(vec![]);
    invalid.schema = "rstyle.source-baseline/2".into();
    assert!(invalid.validate().is_err());
    invalid.schema = SCHEMA.into();
    invalid.policy.spacings.as_mut().unwrap().min_statements = 0;
    assert!(invalid.validate().is_err());
}

#[test]
fn disabled_spacing_is_explicit_and_cannot_retain_positive_spacing() {
    let mut disabled = policy();
    disabled.spacings = None;
    let original = Baseline::new(vec![], disabled.clone(), vec![context()], vec![]).unwrap();
    let wire = serde_json::to_value(&original).unwrap();
    assert!(wire["policy"]["spacings"].is_null());
    assert!(Baseline::new(findings(), disabled, vec![context()], vec![]).is_err());
}

#[test]
fn invalid_arithmetic_and_nonpositive_findings_are_rejected() {
    for index in 0..3 {
        let mut entries = findings();
        match &mut entries[index] {
            Finding::Parameter(value) => value.signature.actual = usize::MAX,
            Finding::Length(value) => value.measurement.code_lines = 10,
            Finding::Spacing(value) => value.measurement.direct_statements = 2,
        }
        assert!(Baseline::new(entries, policy(), vec![context()], vec![]).is_err());
    }
    let mut entries = findings();
    if let Finding::Parameter(value) = &mut entries[0] {
        value.signature.parameters = usize::MAX;
        value.signature.actual = usize::MAX;
    }
    assert!(Baseline::new(entries, policy(), vec![context()], vec![]).is_err());
}

#[test]
fn valid_entries_must_match_each_family_policy_and_versions() {
    for index in 0..3 {
        for change_version in [false, true] {
            let mut entries = findings();
            match &mut entries[index] {
                Finding::Parameter(value) if change_version => value.versions.detector_version += 1,
                Finding::Parameter(value) => value.policy.max_params -= 1,
                Finding::Length(value) if change_version => value.versions.detector_version += 1,
                Finding::Length(value) => value.policy.max_lines -= 1,
                Finding::Spacing(value) if change_version => value.versions.detector_version += 1,
                Finding::Spacing(value) => value.policy.min_statements -= 1,
            }
            assert!(Baseline::new(entries, policy(), vec![context()], vec![]).is_err());
        }
    }
}

#[test]
fn duplicates_are_per_family_locator_and_compilation_target() {
    for original in findings() {
        let mut duplicate = original.clone();
        match &mut duplicate {
            Finding::Parameter(value) => value.context.debug_assertions = false,
            Finding::Length(value) => value.context.debug_assertions = false,
            Finding::Spacing(value) => value.context.debug_assertions = false,
        }
        let contexts = vec![
            original.identity().1.clone(),
            duplicate.identity().1.clone(),
        ];
        assert!(Baseline::new(vec![original, duplicate], policy(), contexts, vec![]).is_err());
    }
    assert!(baseline(findings()).validate().is_ok());
    let mut entries = findings();
    let Finding::Parameter(value) = &mut entries[0] else {
        unreachable!()
    };
    value.context.compilation_target.test = true;
    let contexts = entries
        .iter()
        .map(|value| value.identity().1.clone())
        .collect();
    assert!(Baseline::new(entries, policy(), contexts, vec![]).is_ok());
}

#[test]
fn distinct_targets_of_the_same_rule_coexist_but_missing_identity_fails() {
    let Finding::Parameter(original) = findings().remove(0) else {
        unreachable!()
    };
    let mut other = original.clone();
    other.context.compilation_target.test = true;
    let contexts = vec![original.context.clone(), other.context.clone()];
    let entries = vec![Finding::Parameter(original), Finding::Parameter(other)];
    let mut valid = Baseline::new(entries, policy(), contexts, vec![]).unwrap();
    let Finding::Parameter(value) = &mut valid.entries[0] else {
        unreachable!()
    };
    value.locator.source.clear();
    assert!(valid.validate().is_err());
}

#[test]
fn entry_limit_counts_all_families_together() {
    let entries: Vec<_> = (0..=MAX_ENTRIES)
        .map(|index| {
            let mut entry = findings().remove(index % 3);
            match &mut entry {
                Finding::Parameter(value) => {
                    value.locator.qualified_name = format!("crate::f_{index}")
                }
                Finding::Length(value) => {
                    value.locator.qualified_name = format!("crate::f_{index}")
                }
                Finding::Spacing(value) => {
                    value.locator.qualified_name = format!("crate::f_{index}")
                }
            }
            entry
        })
        .collect();
    assert!(baseline(entries[..MAX_ENTRIES].to_vec()).validate().is_ok());
    assert!(Baseline::new(entries, policy(), vec![context()], vec![]).is_err());
}

#[test]
fn contexts_are_checked_canonical_bounded_and_required_for_findings() {
    assert!(Baseline::new(findings(), policy(), vec![], vec![]).is_err());
    let mut invalid = context();
    invalid.compiler_commit.clear();
    assert!(Baseline::new(vec![], policy(), vec![invalid], vec![]).is_err());
    let mut canonical =
        Baseline::new(vec![], policy(), vec![context(), context()], vec![]).unwrap();
    assert_eq!(canonical.contexts.len(), 1);
    canonical.contexts.push(context());
    assert!(canonical.validate().is_err());
    canonical.contexts = (0..=MAX_ENTRIES)
        .map(|index| {
            let mut value = context();
            value.compiler_commit = format!("compiler_{index:05}");
            value
        })
        .collect();
    assert!(canonical.validate().is_err());
    canonical.contexts.pop();
    canonical.validate().unwrap();
    canonical.contexts.reverse();
    assert!(canonical.validate().is_err());
}

#[test]
fn exclusions_are_validated_even_for_empty_inventories() {
    assert!(Baseline::new(vec![], policy(), vec![], vec!["[".into()]).is_err());
    let mut valid = Baseline::new(
        vec![],
        policy(),
        vec![],
        vec!["target/**".into(); MAX_ENTRIES],
    )
    .unwrap();
    valid.exclude.push("generated/**".into());
    assert!(valid.validate().is_err());
}
