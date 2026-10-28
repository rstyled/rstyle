use super::*;

fn context() -> Context {
    Context {
        compiler_commit: "fixture-compiler-commit".into(),
        edition: "2024".into(),
        data_layout: "fixture-native-layout".into(),
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
        extraction_policy: "hir-data-v3/3.130".into(),
        matching_policy: MatchingPolicy {
            profile: "strict-data/1".into(),
            minimum_size_ratio: "4/5".into(),
            numerical_threshold: "9/10".into(),
            string_threshold: "1/2".into(),
            final_threshold: "9/10".into(),
            scoring: "smaller_graph_overlap".into(),
        },
        consumer_policy: ConsumerPolicy {
            id: "scalar-expression-helper".into(),
            version: 1,
            min_builtin_operations: 2,
            max_literal_holes: 1,
        },
    }
}

fn member(name: &str) -> MemberSnapshot {
    MemberSnapshot {
        key: MemberKey {
            locator: Locator {
                package: Package {
                    identity: "crates/sample/Cargo.toml".into(),
                    name: "sample".into(),
                    version: "1.0.0".into(),
                },
                source: "src/lib.rs".into(),
                qualified_name: format!("sample::{name}"),
                declaration_kind: "free-function".into(),
                discriminator: "unique-declaration".into(),
            },
            context: context(),
        },
        signature: Signature {
            parameters: vec![ScalarType::U32],
            result: ScalarType::U32,
        },
        expression: Expression::Binary {
            ty: ScalarType::U32,
            operator: BinaryOperator::Add,
            left: Box::new(Expression::Formal {
                position: 0,
                ty: ScalarType::U32,
            }),
            right: Box::new(Expression::Literal {
                ty: ScalarType::U32,
                value: "1".into(),
                spelling: "1".into(),
            }),
        },
    }
}

fn finding(names: &[&str]) -> FindingSnapshot {
    let mut members: Vec<_> = names.iter().map(|name| member(name)).collect();
    members.sort();
    FindingSnapshot {
        versions: Versions {
            detector: "scalar-expression-helper".into(),
            detector_version: 1,
            identity_version: 1,
            locator_version: 1,
            context_version: 1,
        },
        members,
    }
}

fn live(snapshot: &FindingSnapshot) -> LiveFinding<'_> {
    let mut pairs = vec![];
    for left in 0..snapshot.members.len() {
        for right in left + 1..snapshot.members.len() {
            pairs.push((&snapshot.members[left].key, &snapshot.members[right].key));
        }
    }
    LiveFinding::checked(snapshot, &pairs).unwrap()
}

fn dismissed(snapshot: &FindingSnapshot, policy: MembershipPolicy) -> ReviewStore {
    let mut store = ReviewStore::default();
    store
        .record_dismissal(&live(snapshot), policy, "reviewed manually".into())
        .unwrap();
    store
}

fn stale_reason_for(store: &ReviewStore, current: &FindingSnapshot) -> StaleReason {
    let Lookup::Stale(entries) = store.lookup(ReviewInput::Checked(&live(current))) else {
        panic!("expected a visible stale review");
    };
    assert_eq!(entries[0].entry.reason, "reviewed manually");
    entries[0].reason
}

#[test]
fn line_movement_remains_reviewed_without_location_identity() {
    let snapshot = finding(&["a", "b"]);
    let store = dismissed(&snapshot, MembershipPolicy::Exact);
    // Presentation lines can move without changing any persisted identity field.
    for _presentation_line in [10, 30] {
        assert!(matches!(
            store.lookup(ReviewInput::Checked(&live(&snapshot))),
            Lookup::HiddenExact(_)
        ));
    }
}

#[test]
fn changed_operation_stales_and_preserves_old_reason() {
    let previous = finding(&["a", "b"]);
    let store = dismissed(&previous, MembershipPolicy::Exact);
    let mut current = previous.clone();
    let Expression::Binary { operator, .. } = &mut current.members[0].expression else {
        unreachable!()
    };
    *operator = BinaryOperator::Subtract;
    assert_eq!(stale_reason_for(&store, &current), StaleReason::Body);
    assert_eq!(store.entries[0].finding, previous);
}

#[test]
fn duplicate_compilation_contexts_remain_separate_and_visible() {
    let previous = finding(&["a", "b"]);
    let store = dismissed(&previous, MembershipPolicy::Exact);
    let mut current = previous.clone();
    for member in &mut current.members {
        member.key.context.compilation_target.test = true;
        member.key.context.features = vec!["extra".into()];
    }
    assert_eq!(stale_reason_for(&store, &current), StaleReason::Context);
    assert!(matches!(
        store.lookup(ReviewInput::Checked(&live(&previous))),
        Lookup::HiddenExact(_)
    ));
}

#[test]
fn same_short_name_does_not_merge_qualified_owners() {
    let previous = finding(&["left::run", "left::second"]);
    let current = finding(&["right::run", "right::second"]);
    let store = dismissed(&previous, MembershipPolicy::Exact);
    assert!(matches!(
        store.lookup(ReviewInput::Checked(&live(&current))),
        Lookup::VisibleNew
    ));
}

#[test]
fn equal_external_selector_hints_never_authorize_exact_lookup() {
    // Hints exist only in this fixture wrapper; the persisted mechanism has no hash.
    let previous = ("xxh3-64:0000000000000000", finding(&["a", "b"]));
    let current = ("xxh3-64:0000000000000000", finding(&["c", "d"]));
    assert_eq!(previous.0, current.0);
    let mut store = dismissed(&previous.1, MembershipPolicy::Exact);
    assert!(matches!(
        store.lookup(ReviewInput::Checked(&live(&current.1))),
        Lookup::VisibleNew
    ));
    store
        .record_dismissal(
            &live(&current.1),
            MembershipPolicy::Exact,
            "separate".into(),
        )
        .unwrap();
    assert_eq!(store.entries.len(), 2);
}

#[test]
fn fixed_membership_growth_stales() {
    let store = dismissed(&finding(&["a", "b"]), MembershipPolicy::Exact);
    assert_eq!(
        stale_reason_for(&store, &finding(&["a", "b", "c"])),
        StaleReason::Membership
    );
}

#[test]
fn explicit_growth_requires_unchanged_original_members() {
    let previous = finding(&["a", "b"]);
    let store = dismissed(&previous, MembershipPolicy::AllowGrowth);
    let current = finding(&["a", "b", "c"]);
    let Lookup::HiddenGrowth(entry) = store.lookup(ReviewInput::Checked(&live(&current))) else {
        panic!("explicit growth should remain reviewed")
    };
    assert_eq!(entry.reason, "reviewed manually");
    let mut changed = current;
    changed.members[0]
        .signature
        .parameters
        .push(ScalarType::U32);
    assert_eq!(stale_reason_for(&store, &changed), StaleReason::Membership);
}

#[test]
fn growth_permission_does_not_authorize_member_loss() {
    let store = dismissed(&finding(&["a", "b"]), MembershipPolicy::AllowGrowth);
    assert_eq!(
        stale_reason_for(&store, &finding(&["a", "c"])),
        StaleReason::Membership
    );
}

#[test]
fn version_and_effective_policy_changes_need_review() {
    let previous = finding(&["a", "b"]);
    let store = dismissed(&previous, MembershipPolicy::Exact);
    for index in 0..4 {
        let mut current = previous.clone();
        match index {
            0 => current.versions.detector_version += 1,
            1 => current.versions.identity_version += 1,
            2 => current.versions.locator_version += 1,
            _ => current.versions.context_version += 1,
        }
        assert_eq!(stale_reason_for(&store, &current), StaleReason::Versions);
    }
    for extraction_change in [false, true] {
        let mut current = previous.clone();
        for member in &mut current.members {
            if extraction_change {
                member.key.context.extraction_policy = "changed-policy".into();
            } else {
                member.key.context.consumer_policy.max_literal_holes = 2;
            }
        }
        assert_eq!(stale_reason_for(&store, &current), StaleReason::Context);
    }
}

#[test]
fn connected_edges_cannot_mint_live_growth_capability() {
    let snapshot = finding(&["a", "b", "c"]);
    let members = &snapshot.members;
    let connected = [
        (&members[0].key, &members[1].key),
        (&members[1].key, &members[2].key),
    ];
    assert!(matches!(
        LiveFinding::checked(&snapshot, &connected),
        Err(ReviewError::IncompletePairRoster)
    ));
    assert!(LiveFinding::checked(&snapshot, &[]).is_err());
    assert!(live(&snapshot).snapshot == &snapshot);
}

#[test]
fn ambiguous_locators_cannot_merge_even_across_contexts() {
    let mut snapshot = finding(&["a", "b"]);
    snapshot.members[1].key.locator = snapshot.members[0].key.locator.clone();
    snapshot.members[1].key.context.compilation_target.test = true;
    assert!(matches!(
        LiveFinding::checked(&snapshot, &[]),
        Err(ReviewError::AmbiguousLocator)
    ));
}

#[test]
fn persistence_roundtrip_has_no_proof_and_rejects_extra_fields() {
    let snapshot = finding(&["a", "b"]);
    let store = dismissed(&snapshot, MembershipPolicy::AllowGrowth);
    let bytes = store.to_json().unwrap();
    let loaded = ReviewStore::from_json(&bytes).unwrap();
    assert_eq!(store, loaded);
    let mut value: serde_json::Value = serde_json::from_str(&bytes).unwrap();
    value["entries"][0]["finding"]["all_member_pairs_checked"] = true.into();
    assert!(ReviewStore::from_json(&value.to_string()).is_err());
    assert!(matches!(
        loaded.lookup(ReviewInput::Unavailable {
            reason: "missing fresh native proposal"
        }),
        Lookup::Unavailable { .. }
    ));
}

#[test]
fn self_duplicate_and_foreign_roster_edges_are_rejected() {
    let snapshot = finding(&["a", "b"]);
    let left = &snapshot.members[0].key;
    let right = &snapshot.members[1].key;
    let foreign = member("foreign");
    for pairs in [
        vec![(left, left)],
        vec![(left, right), (right, left)],
        vec![(left, &foreign.key)],
    ] {
        assert!(matches!(
            LiveFinding::checked(&snapshot, &pairs),
            Err(ReviewError::InvalidPairRoster)
        ));
    }
}

#[test]
fn malformed_persisted_schema_and_package_relative_owners_are_unavailable() {
    let snapshot = finding(&["a", "b"]);
    let mut store = dismissed(&snapshot, MembershipPolicy::Exact);
    store.schema_version = 99;
    assert!(store.to_json().is_err());
    assert!(matches!(
        store.lookup(ReviewInput::Checked(&live(&snapshot))),
        Lookup::Unavailable { .. }
    ));
    for source in ["/src/lib.rs", "../src/lib.rs", "src/./lib.rs"] {
        let mut malformed = snapshot.clone();
        malformed.members[0].key.locator.source = source.into();
        assert!(matches!(
            LiveFinding::checked(&malformed, &[]),
            Err(ReviewError::InvalidLocator)
        ));
    }
}

#[test]
fn formal_positions_and_signature_parameter_budget_are_checked() {
    let mut member = member("a");
    member.expression = Expression::Formal {
        position: 1,
        ty: ScalarType::U32,
    };
    assert_eq!(
        validate_member_expression(&member),
        Err(ReviewError::InvalidExpression)
    );
    member.signature.parameters = vec![ScalarType::U32; 64];
    member.expression = Expression::Formal {
        position: 63,
        ty: ScalarType::U32,
    };
    assert!(validate_member_expression(&member).is_ok());
    member.signature.parameters.push(ScalarType::U32);
    assert_eq!(
        validate_member_expression(&member),
        Err(ReviewError::InvalidSignature)
    );
}

#[test]
fn literal_semantics_are_canonical_u32_and_match_retained_spelling() {
    for (value, spelling) in [
        ("0", "0"),
        ("1", "01u32"),
        ("255", "0xffu32"),
        ("8", "0o10"),
        ("3", "0b11"),
        ("4294967295", "0xffff_ffffu32"),
    ] {
        assert!(valid_literal(value, spelling), "{value}: {spelling}");
    }
    for (value, spelling) in [
        ("01", "1"),
        ("4294967296", "4294967296"),
        ("-1", "-1"),
        ("+1", "1"),
        ("1", "2"),
        ("1", "1u64"),
        ("1", "0x+1"),
        ("1", "_1"),
        ("1", ""),
        ("", "0"),
    ] {
        assert!(!valid_literal(value, spelling), "{value}: {spelling}");
    }
}

fn wrap(expression: Expression) -> Expression {
    Expression::Unary {
        ty: ScalarType::U32,
        operator: UnaryOperator::Not,
        operand: Box::new(expression),
    }
}

fn balanced(depth: usize) -> Expression {
    if depth == 0 {
        return Expression::Formal {
            position: 0,
            ty: ScalarType::U32,
        };
    }
    Expression::Binary {
        ty: ScalarType::U32,
        operator: BinaryOperator::BitOr,
        left: Box::new(balanced(depth - 1)),
        right: Box::new(balanced(depth - 1)),
    }
}

#[test]
fn expression_depth_and_node_budgets_have_exact_boundaries() {
    let mut member = member("a");
    member.expression = balanced(0);
    for _ in 0..63 {
        member.expression = wrap(member.expression);
    }
    assert!(validate_member_expression(&member).is_ok());
    member.expression = wrap(member.expression);
    assert_eq!(
        validate_member_expression(&member),
        Err(ReviewError::ExpressionBudget)
    );
    member.expression = wrap(balanced(8)); // 511 tree nodes plus one unary.
    assert!(validate_member_expression(&member).is_ok());
    member.expression = wrap(member.expression);
    assert_eq!(
        validate_member_expression(&member),
        Err(ReviewError::ExpressionBudget)
    );
}

#[test]
fn semantic_equal_literal_spelling_edit_deliberately_stales_review() {
    let previous = finding(&["a", "b"]);
    let store = dismissed(&previous, MembershipPolicy::Exact);
    let mut current = previous.clone();
    let Expression::Binary { right, .. } = &mut current.members[0].expression else {
        unreachable!()
    };
    let Expression::Literal { spelling, .. } = &mut **right else {
        unreachable!()
    };
    *spelling = "0x1u32".into();
    assert_eq!(stale_reason_for(&store, &current), StaleReason::Body);
}

#[test]
fn run_specific_package_roots_and_opaque_targets_cannot_load() {
    let snapshot = finding(&["a", "b"]);
    let mut malformed = snapshot.clone();
    malformed.members[0].key.locator.package.identity = "/tmp/run123/sample/Cargo.toml".into();
    assert!(matches!(
        LiveFinding::checked(&malformed, &[]),
        Err(ReviewError::InvalidLocator)
    ));
    let store = dismissed(&snapshot, MembershipPolicy::Exact);
    let mut value: serde_json::Value = serde_json::from_str(&store.to_json().unwrap()).unwrap();
    value["entries"][0]["finding"]["members"][0]["key"]["context"]["compilation_target"] =
        "lib".into();
    assert!(ReviewStore::from_json(&value.to_string()).is_err());
}

#[test]
fn matching_threshold_changes_are_relevant_and_unknown_scoring_is_invalid() {
    let previous = finding(&["a", "b"]);
    let store = dismissed(&previous, MembershipPolicy::Exact);
    let mut current = previous.clone();
    for member in &mut current.members {
        member.key.context.matching_policy.final_threshold = "1/1".into();
    }
    assert_eq!(stale_reason_for(&store, &current), StaleReason::Context);
    current.members[0].key.context.matching_policy.scoring = "unknown".into();
    assert!(matches!(
        LiveFinding::checked(&current, &[]),
        Err(ReviewError::InvalidContext)
    ));
}

#[test]
fn checked_store_view_reuses_immutable_review_entries() {
    let snapshot = finding(&["a", "b"]);
    let store = dismissed(&snapshot, MembershipPolicy::Exact);
    let checked = store.checked().unwrap();
    let current = live(&snapshot);
    for _ in 0..2 {
        let Lookup::HiddenExact(entry) = checked.lookup(ReviewInput::Checked(&current)) else {
            panic!("checked review should remain exact");
        };
        assert!(std::ptr::eq(entry, &store.entries[0]));
    }
    assert!(matches!(
        checked.lookup(ReviewInput::Unavailable {
            reason: "no live native proof"
        }),
        Lookup::Unavailable { .. }
    ));
    let new_snapshot = finding(&["c", "d"]);
    assert!(matches!(
        checked.lookup(ReviewInput::Checked(&live(&new_snapshot))),
        Lookup::VisibleNew
    ));
}

#[test]
fn empty_reasons_cannot_enter_or_load_a_review_store() {
    let snapshot = finding(&["a", "b"]);
    let current = live(&snapshot);
    let mut store = ReviewStore::default();
    assert_eq!(
        store.record_dismissal(&current, MembershipPolicy::Exact, "  ".into()),
        Err(ReviewError::InvalidReason)
    );
    assert!(store.entries.is_empty());
    let valid = dismissed(&snapshot, MembershipPolicy::Exact);
    let mut bytes: serde_json::Value = serde_json::from_str(&valid.to_json().unwrap()).unwrap();
    bytes["entries"][0]["reason"] = "".into();
    assert!(ReviewStore::from_json(&bytes.to_string()).is_err());
}

#[test]
fn helper_store_wire_shape_is_preserved_and_future_envelopes_are_rejected() {
    let store = dismissed(&finding(&["a", "b"]), MembershipPolicy::Exact);
    let bytes = store.to_json().unwrap();
    let mut wire: serde_json::Value = serde_json::from_str(&bytes).unwrap();
    assert_eq!(wire["schema_version"], 1);
    assert!(wire.get("source_entries").is_none());
    assert_eq!(ReviewStore::from_json(&bytes).unwrap(), store);
    wire["schema_version"] = serde_json::json!(5);
    assert!(ReviewStore::from_json(&wire.to_string()).is_err());
    wire["schema_version"] = serde_json::json!(2);
    let v2 = ReviewStore::from_json(&wire.to_string()).unwrap();
    assert_eq!(v2.entries, store.entries);
    assert!(v2.source_entries.is_empty());
}
