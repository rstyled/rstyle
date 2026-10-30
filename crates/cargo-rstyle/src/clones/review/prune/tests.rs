use super::*;
use crate::clones::review::*;

fn locator(name: &str) -> Locator {
    Locator {
        package: Package {
            identity: "Cargo.toml".into(),
            name: "sample".into(),
            version: "1.0.0".into(),
        },
        source: "src/lib.rs".into(),
        qualified_name: format!("sample::{name}"),
        declaration_kind: "free-function".into(),
        discriminator: "unique-declaration".into(),
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
        cfgs: vec![],
        opt_level: "0".into(),
        debug_assertions: true,
        overflow_checks: true,
        panic_strategy: "unwind".into(),
        extraction_policy: "source-local/1".into(),
    }
}

fn versions(detector: &str) -> Versions {
    Versions {
        detector: detector.into(),
        detector_version: 1,
        identity_version: 1,
        locator_version: 1,
        context_version: 1,
    }
}

fn helper(index: usize) -> ReviewEntry {
    let mut value = serde_json::to_value(context()).unwrap();
    value["matching_policy"] = serde_json::json!({
        "profile": "strict-data/1", "minimum_size_ratio": "1", "numerical_threshold": "1",
        "string_threshold": "1", "final_threshold": "1", "scoring": "smaller_graph_overlap"
    });
    value["consumer_policy"] = serde_json::json!({
        "id": "scalar-expression-helper", "version": 1,
        "min_builtin_operations": 2, "max_literal_holes": 1
    });
    let context: Context = serde_json::from_value(value).unwrap();
    ReviewEntry {
        finding: FindingSnapshot {
            versions: versions("scalar-expression-helper"),
            members: ["a", "b"]
                .map(|suffix| MemberSnapshot {
                    key: MemberKey {
                        locator: locator(&format!("helper_{index}_{suffix}")),
                        context: context.clone(),
                    },
                    signature: Signature {
                        parameters: vec![ScalarType::U32],
                        result: ScalarType::U32,
                    },
                    expression: Expression::Formal {
                        position: 0,
                        ty: ScalarType::U32,
                    },
                })
                .into(),
        },
        membership: MembershipPolicy::Exact,
        reason: format!("helper reason {index}"),
    }
}

fn source(index: usize, dispatch: bool) -> local::Entry {
    local::Entry {
        finding: local::Snapshot {
            versions: versions(if dispatch {
                "local-enum-dispatch"
            } else {
                "ordered-local-capability"
            }),
            members: vec![local::Member {
                locator: locator(&format!("source_{index}")),
                context: context(),
                source: "fn wrapper() {}".into(),
            }],
            proposal: if dispatch {
                local::Proposal::EnumDispatch {
                    enumeration: local::Declaration {
                        locator: locator("Choice"),
                        source: "enum Choice {}".into(),
                    },
                    arms: vec![],
                }
            } else {
                local::Proposal::OrderedCapability { roles: vec![] }
            },
        },
        reason: format!("source reason {index}"),
    }
}

fn store() -> ReviewStore {
    ReviewStore {
        schema_version: 2,
        entries: (0..4).map(helper).collect(),
        source_entries: (0..4).map(|index| source(index, index % 2 == 0)).collect(),
        parameter_entries: vec![],
        length_entries: vec![],
        spacing_entries: vec![],
        mutation_entries: vec![],
        density_entries: vec![],
        compiler_pattern_entries: vec![],
        compiler_diagnostic_entries: vec![],
        guard_entries: vec![],
    }
}

fn assessment(store: &ReviewStore, states: &[status::State]) -> status::Report {
    let helpers = store
        .entries
        .iter()
        .enumerate()
        .map(|(entry_index, entry)| status::Record {
            family: status::Family::Helper,
            entry_index,
            state: states[entry_index],
            detail: "completed assessment".into(),
            reason: entry.reason.clone(),
            members: entry
                .finding
                .members
                .iter()
                .map(|member| member.key.locator.clone())
                .collect(),
        });
    let sources = store
        .source_entries
        .iter()
        .enumerate()
        .map(|(entry_index, entry)| status::Record {
            family: match entry.finding.proposal {
                local::Proposal::EnumDispatch { .. } => status::Family::EnumDispatch,
                local::Proposal::OrderedCapability { .. } => status::Family::CapabilitySet,
            },
            entry_index,
            state: states[store.entries.len() + entry_index],
            detail: "completed assessment".into(),
            reason: entry.reason.clone(),
            members: entry
                .finding
                .members
                .iter()
                .map(|member| member.locator.clone())
                .collect(),
        });
    status::Report::new(helpers.chain(sources).collect(), true)
}

#[test]
fn only_completed_negative_entries_are_removed_across_all_families() {
    use status::State::*;
    let mut store = store();
    let original = store.clone();
    let status = assessment(
        &store,
        &[
            NoFinding,
            HiddenExact,
            HiddenGrowth,
            Stale,
            NoFinding,
            NoFinding,
            ContextChanged,
            Unavailable,
        ],
    );
    assert_eq!(status.coverage, Coverage::Incomplete);
    let report = apply(&mut store, &status, false).unwrap();
    assert_eq!(report.eligible_entries, 3);
    assert_eq!(report.removed_entries, 3);
    assert_eq!(report.retained_entries, 5);
    assert_eq!(store.schema_version, 2);
    assert_eq!(store.entries, original.entries[1..]);
    assert_eq!(store.source_entries, original.source_entries[2..]);
    let value = serde_json::to_value(report).unwrap();
    assert_eq!(value["schema"], "rstyle.review-prune/1");
    assert_eq!(
        value["selections"],
        serde_json::json!([
            {"family":"helper", "entry_index":0},
            {"family":"enum_dispatch", "entry_index":0},
            {"family":"capability_set", "entry_index":1}
        ])
    );
}

#[test]
fn dry_run_reports_original_ordinals_without_changing_any_stored_value() {
    let mut store = store();
    let original = store.clone();
    let status = assessment(&store, &[status::State::NoFinding; 8]);
    let report = apply(&mut store, &status, true).unwrap();
    assert_eq!(store, original);
    assert_eq!(report.eligible_entries, 8);
    assert_eq!(report.removed_entries, 0);
    assert_eq!(report.retained_entries, 8);
    assert!(report.render().contains("dry run"));
}

#[test]
fn stale_changed_unobserved_unrequested_unavailable_reviews_are_never_pruned() {
    use status::State::*;
    let mut store = store();
    let original = store.clone();
    let status = assessment(
        &store,
        &[
            HiddenExact,
            HiddenGrowth,
            Stale,
            ContextChanged,
            NotObserved,
            NotRequested,
            Unavailable,
            Stale,
        ],
    );
    let report = apply(&mut store, &status, false).unwrap();
    assert_eq!(store, original);
    assert_eq!(report.eligible_entries, 0);
    assert_eq!(report.removed_entries, 0);
    assert_eq!(report.retained_entries, 8);
    assert!(report.selections.is_empty());
}

fn rejected(status: status::Report) {
    for dry_run in [false, true] {
        let mut store = store();
        let original = store.clone();
        assert!(apply(&mut store, &status, dry_run).is_err());
        assert_eq!(store, original);
    }
}

#[test]
fn duplicate_missing_and_out_of_range_rows_reject_before_any_mutation() {
    let store = store();
    let mut duplicate = assessment(&store, &[status::State::NoFinding; 8]);
    duplicate.records[1].entry_index = 0;
    duplicate.records[1].reason = duplicate.records[0].reason.clone();
    duplicate.records[1].members = duplicate.records[0].members.clone();
    rejected(duplicate);
    let mut missing = assessment(&store, &[status::State::NoFinding; 8]);
    missing.records.pop();
    rejected(missing);
    let mut outside = assessment(&store, &[status::State::NoFinding; 8]);
    outside.records[7].entry_index = 4;
    rejected(outside);
}

#[test]
fn wrong_family_member_order_locator_and_reason_are_rejected_without_mutation() {
    let store = store();
    let mut family = assessment(&store, &[status::State::NoFinding; 8]);
    family.records[5].family = status::Family::EnumDispatch;
    rejected(family);
    let mut members = assessment(&store, &[status::State::NoFinding; 8]);
    members.records[3].members.reverse();
    rejected(members);
    let mut locator = assessment(&store, &[status::State::NoFinding; 8]);
    locator.records[7].members[0].source = "src/other.rs".into();
    rejected(locator);
    let mut reason = assessment(&store, &[status::State::NoFinding; 8]);
    reason.records[7].reason = "another dismissal".into();
    rejected(reason);
}

#[test]
fn uncompleted_and_inconsistent_summary_reports_cannot_remove_entries() {
    let store = store();
    let mut execution = assessment(&store, &[status::State::NoFinding; 8]);
    execution.execution = Execution::NotRequested;
    rejected(execution);
    let mut coverage = assessment(&store, &[status::State::NoFinding; 8]);
    coverage.coverage = Coverage::Incomplete;
    rejected(coverage);
    let mut count = assessment(&store, &[status::State::NoFinding; 8]);
    count.assessed_entries = 7;
    rejected(count);
    let mut schema = assessment(&store, &[status::State::NoFinding; 8]);
    schema.schema = "unknown/1";
    rejected(schema);
}

#[test]
fn empty_and_helper_only_stores_preserve_their_schema_versions() {
    for schema_version in [1, 2, 3, 4, 5] {
        let mut store = ReviewStore {
            schema_version,
            ..ReviewStore::default()
        };
        let status = status::Report::new(Vec::new(), true);
        let report = apply(&mut store, &status, false).unwrap();
        assert_eq!(report.retained_entries, 0);
        assert_eq!(store.schema_version, schema_version);
        store.entries.push(helper(0));
        let status = assessment(&store, &[status::State::NoFinding]);
        let report = apply(&mut store, &status, false).unwrap();
        assert_eq!(report.removed_entries, 1);
        assert!(store.entries.is_empty());
        assert_eq!(store.schema_version, schema_version);
    }
}

#[test]
fn status_row_order_does_not_change_original_storage_order_or_ordinals() {
    let mut store = store();
    let original = store.clone();
    let mut status = assessment(&store, &[status::State::HiddenExact; 8]);
    status.records[2].state = status::State::NoFinding;
    status.records[6].state = status::State::NoFinding;
    status.records.reverse();
    let report = apply(&mut store, &status, false).unwrap();
    assert_eq!(report.removed_entries, 2);
    assert_eq!(
        store.entries,
        [
            original.entries[0].clone(),
            original.entries[1].clone(),
            original.entries[3].clone()
        ]
    );
    assert_eq!(
        store.source_entries,
        [
            original.source_entries[0].clone(),
            original.source_entries[1].clone(),
            original.source_entries[3].clone()
        ]
    );
}
