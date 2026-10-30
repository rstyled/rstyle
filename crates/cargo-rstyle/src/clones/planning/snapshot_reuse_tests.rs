use super::{model::*, overlap::analyze};
use rstyle_pdg::ByteSpan;

const MIB: usize = 1024 * 1024;

fn plan<'a>(id: usize, resource: &'a str, snapshot: &'a str) -> Plan<'a> {
    Plan {
        id,
        required_regions: 1,
        regions: vec![Region {
            resource,
            snapshot,
            span: ByteSpan { start: 0, end: 1 },
        }],
    }
}

#[test]
fn many_regions_of_one_equal_second_owner_pay_for_one_actual_equality_check() {
    let original = "x".repeat(MIB);
    let second = original.clone();
    assert_ne!(original.as_ptr(), second.as_ptr());
    let plans = std::iter::once(plan(0, "source.rs", &original))
        .chain((1..=20).map(|id| plan(id, "source.rs", &second)))
        .collect::<Vec<_>>();
    let result = analyze(&plans, true);
    assert_eq!(result.execution, Execution::Completed);
    assert!(result.coverage_complete);
    assert_eq!(result.snapshot_comparison_bytes, MIB);
    assert!(result.unavailable_plans.is_empty());
    assert_eq!(result.overlap_groups, [(0..=20).collect::<Vec<_>>()]);
}

#[test]
fn independently_allocated_third_owner_requires_its_own_complete_equality_check() {
    let original = "x".repeat(MIB);
    let second = original.clone();
    let third = original.clone();
    let plans = [
        plan(0, "source.rs", &original),
        plan(1, "source.rs", &second),
        plan(2, "source.rs", &third),
        plan(3, "source.rs", &second),
        plan(4, "source.rs", &third),
    ];
    let result = analyze(&plans, true);
    assert!(result.coverage_complete);
    assert_eq!(result.snapshot_comparison_bytes, 2 * MIB);
}

#[test]
fn equality_observations_are_local_to_the_physical_source_resource() {
    let original = "x".repeat(MIB);
    let second = original.clone();
    let plans = [
        plan(0, "left.rs", &original),
        plan(1, "left.rs", &second),
        plan(2, "right.rs", &original),
        plan(3, "right.rs", &second),
    ];
    let result = analyze(&plans, true);
    assert!(result.coverage_complete);
    assert_eq!(result.snapshot_comparison_bytes, 2 * MIB);
    assert_eq!(result.overlap_groups, [vec![0, 1], vec![2, 3]]);
}

#[test]
fn later_changed_owner_invalidates_every_plan_including_previously_equal_owner() {
    let original = "x".repeat(MIB);
    let second = original.clone();
    let changed = format!("{}y", "x".repeat(MIB - 1));
    let plans = [
        plan(0, "source.rs", &original),
        plan(1, "source.rs", &second),
        plan(2, "source.rs", &second),
        plan(3, "source.rs", &changed),
        plan(4, "source.rs", &second),
        plan(5, "source.rs", &changed),
    ];
    let result = analyze(&plans, true);
    assert_eq!(result.execution, Execution::Completed);
    assert!(!result.coverage_complete);
    assert_eq!(result.snapshot_comparison_bytes, 2 * MIB);
    assert_eq!(result.conflicting_snapshot_plans, [0, 1, 2, 3, 4, 5]);
    assert!(
        result
            .relations
            .iter()
            .all(|pair| pair.relation == Relation::ConflictingSnapshots)
    );
    assert!(result.overlap_groups.is_empty());
}

#[test]
fn exhausted_comparison_cannot_be_bypassed_by_a_previously_equal_owner() {
    let original = "x".repeat(9 * MIB);
    let second = original.clone();
    let unverified = original.clone();
    let plans = [
        plan(0, "source.rs", &original),
        plan(1, "source.rs", &second),
        plan(2, "source.rs", &unverified),
        plan(3, "source.rs", &second),
        plan(4, "source.rs", &unverified),
        plan(5, "source.rs", &original),
    ];
    let result = analyze(&plans, true);
    assert_eq!(result.execution, Execution::BudgetExhausted);
    assert!(!result.coverage_complete);
    assert_eq!(result.snapshot_comparison_bytes, 9 * MIB);
    assert_eq!(result.unavailable_plans, [0, 1, 2, 3, 4, 5]);
    assert!(
        result
            .relations
            .iter()
            .all(|pair| pair.relation == Relation::Unavailable)
    );
    assert!(result.overlap_groups.is_empty());
}

#[test]
fn equal_slices_at_distinct_offsets_of_one_allocation_still_require_equality() {
    let owner = "x".repeat(2 * MIB);
    let first = &owner[..MIB];
    let second = &owner[MIB..];
    assert_ne!(first.as_ptr(), second.as_ptr());
    let plans = [
        plan(0, "source.rs", first),
        plan(1, "source.rs", second),
        plan(2, "source.rs", second),
    ];
    let result = analyze(&plans, true);
    assert!(result.coverage_complete);
    assert_eq!(result.snapshot_comparison_bytes, MIB);
}

#[test]
fn same_allocation_offsets_and_slice_lengths_cannot_hide_changed_snapshots() {
    let owner = "xxxxy";
    let changed = [
        plan(0, "source.rs", &owner[..4]),
        plan(1, "source.rs", &owner[1..]),
    ];
    let result = analyze(&changed, true);
    assert_eq!(result.snapshot_comparison_bytes, 4);
    assert_eq!(result.conflicting_snapshot_plans, [0, 1]);
    let shortened = [
        plan(0, "source.rs", &owner[..4]),
        plan(1, "source.rs", &owner[..3]),
    ];
    let result = analyze(&shortened, true);
    assert_eq!(result.snapshot_comparison_bytes, 0);
    assert_eq!(result.conflicting_snapshot_plans, [0, 1]);
}

#[test]
fn a_fresh_analysis_rechecks_equal_owners_instead_of_retaining_old_proof() {
    let original = "x".repeat(MIB);
    let second = original.clone();
    let plans = [
        plan(0, "source.rs", &original),
        plan(1, "source.rs", &second),
    ];
    for _ in 0..2 {
        let result = analyze(&plans, true);
        assert!(result.coverage_complete);
        assert_eq!(result.snapshot_comparison_bytes, MIB);
    }
}
