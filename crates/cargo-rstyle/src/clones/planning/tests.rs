use super::{model::*, overlap::analyze};
use rstyle_pdg::ByteSpan;

fn region(resource: &'static str, start: u32, end: u32) -> Region<'static> {
    Region {
        resource,
        snapshot: "retained exact snapshot",
        span: ByteSpan { start, end },
    }
}

fn plan(id: usize, first: Region<'static>, second: Region<'static>) -> Plan<'static> {
    Plan {
        id,
        required_regions: 2,
        regions: vec![first, second],
    }
}

#[test]
fn touching_half_open_regions_are_disjoint_and_nested_regions_intersect() {
    let plans = [
        plan(0, region("src/lib.rs", 10, 20), region("left.rs", 0, 10)),
        plan(1, region("src/lib.rs", 20, 30), region("right.rs", 0, 10)),
        plan(2, region("src/lib.rs", 12, 18), region("other.rs", 0, 10)),
    ];
    let result = analyze(&plans, true);
    assert!(result.coverage_complete);
    assert_eq!(result.relations[0].relation, Relation::DisjointKnownRegions);
    assert_eq!(
        result.relations[1].relation,
        Relation::OverlapWitness {
            witnesses: vec![Witness {
                first_region: 0,
                second_region: 0,
                intersection: ByteSpan { start: 12, end: 18 },
            }],
        }
    );
    assert_eq!(result.overlap_groups, [vec![0, 2]]);
}

#[test]
fn equal_names_and_bytes_do_not_merge_distinct_files() {
    let plans = [
        plan(0, region("left.rs", 0, 20), region("left.rs", 30, 40)),
        plan(1, region("right.rs", 0, 20), region("right.rs", 30, 40)),
    ];
    let result = analyze(&plans, true);
    assert_eq!(result.relations[0].relation, Relation::DisjointKnownRegions);
    assert!(result.overlap_groups.is_empty());
}

#[test]
fn distinct_plans_still_share_the_same_physical_edit() {
    let lib = region("src/lib.rs", 10, 20);
    let test = region("src/lib.rs", 10, 20);
    let plans = [
        plan(7, lib, region("other.rs", 0, 10)),
        plan(2, test, region("elsewhere.rs", 0, 10)),
    ];
    let result = analyze(&plans, true);
    assert!(matches!(
        result.relations[0].relation,
        Relation::OverlapWitness { .. }
    ));
    assert_eq!(result.overlap_groups, [vec![2, 7]]);
}

#[test]
fn unequal_snapshots_prevent_shared_coordinates_even_for_disjoint_ranges() {
    let mut changed = region("src/lib.rs", 30, 40);
    changed.snapshot = "different exact snapshot";
    let plans = [
        plan(0, region("src/lib.rs", 10, 20), region("other.rs", 0, 10)),
        plan(1, changed, region("elsewhere.rs", 0, 10)),
    ];
    let result = analyze(&plans, true);
    assert_eq!(result.relations[0].relation, Relation::ConflictingSnapshots);
    assert!(!result.coverage_complete);
    assert!(result.overlap_groups.is_empty());
}

#[test]
fn three_pair_proposals_form_one_coordination_group_with_exact_witnesses() {
    let plans = [
        plan(0, region("src/lib.rs", 0, 10), region("src/lib.rs", 20, 30)),
        plan(1, region("src/lib.rs", 0, 10), region("src/lib.rs", 40, 50)),
        plan(
            2,
            region("src/lib.rs", 20, 30),
            region("src/lib.rs", 40, 50),
        ),
    ];
    let result = analyze(&plans, false);
    assert_eq!(result.overlap_groups, [vec![0, 1, 2]]);
    assert_eq!(result.assessed_comparisons, 3);
    assert_eq!(result.expected_comparisons, None);
    assert!(!result.coverage_complete);
    for pair in result.relations {
        let Relation::OverlapWitness { witnesses } = pair.relation else {
            panic!("every pair shares one original body");
        };
        assert_eq!(witnesses.len(), 1);
    }
}

#[test]
fn missing_member_region_is_unavailable_even_without_other_plans() {
    let mut incomplete = plan(0, region("a.rs", 0, 10), region("b.rs", 0, 10));
    incomplete.regions.pop();
    assert!(!analyze(&[incomplete], true).coverage_complete);
    let plans = [
        Plan {
            id: 0,
            required_regions: 2,
            regions: vec![],
        },
        plan(1, region("a.rs", 0, 10), region("b.rs", 0, 10)),
    ];
    assert_eq!(
        analyze(&plans, true).relations[0].relation,
        Relation::Unavailable
    );
}

#[test]
fn work_budget_keeps_witnessed_overlaps_and_declares_unassessed_remainder() {
    let plans = (0..200)
        .map(|id| plan(id, region("shared.rs", 0, 10), region("shared.rs", 20, 30)))
        .collect::<Vec<_>>();
    let result = analyze(&plans, true);
    assert_eq!(result.execution, Execution::BudgetExhausted);
    assert_eq!(result.expected_comparisons, Some(19_900));
    assert_eq!(result.assessed_comparisons, MAX_COMPARISONS / 4);
    assert_eq!(result.region_comparisons, MAX_COMPARISONS);
    assert!(!result.coverage_complete);
    assert!(!result.overlap_groups.is_empty());
    assert!(
        result
            .relations
            .iter()
            .all(|pair| matches!(pair.relation, Relation::OverlapWitness { .. }))
    );
}

#[test]
fn plan_population_budget_preserves_incomplete_status() {
    let plans = (0..MAX_PLANS + 1)
        .map(|id| Plan {
            id,
            required_regions: 2,
            regions: vec![],
        })
        .collect::<Vec<_>>();
    let result = analyze(&plans, true);
    assert_eq!(result.execution, Execution::BudgetExhausted);
    assert_eq!(result.assessed_comparisons, MAX_COMPARISONS);
    assert!(!result.coverage_complete);
}

#[test]
fn unequal_snapshots_inside_one_plan_are_explicitly_incomplete() {
    let mut changed = region("src/lib.rs", 30, 40);
    changed.snapshot = "different exact snapshot";
    let plans = [plan(4, region("src/lib.rs", 10, 20), changed)];
    let result = analyze(&plans, true);
    assert!(!result.coverage_complete);
    assert_eq!(result.conflicting_snapshot_plans, [4]);
    assert!(result.relations.is_empty());
    assert_eq!(result.expected_comparisons, Some(0));
}

#[test]
fn snapshot_byte_budget_limits_exact_equality_work_and_preserves_other_resources() {
    let copies = vec!["x".repeat(1024 * 1024); 20];
    let mut plans: Vec<_> = copies
        .iter()
        .enumerate()
        .map(|(id, bytes)| Plan {
            id,
            required_regions: 2,
            regions: vec![
                Region {
                    resource: "large.rs",
                    snapshot: bytes,
                    span: ByteSpan { start: 0, end: 10 },
                },
                Region {
                    resource: "large.rs",
                    snapshot: bytes,
                    span: ByteSpan { start: 20, end: 30 },
                },
            ],
        })
        .collect();
    plans.extend([
        plan(100, region("small.rs", 0, 10), region("small.rs", 20, 30)),
        plan(101, region("small.rs", 0, 10), region("small.rs", 20, 30)),
    ]);
    let result = analyze(&plans, true);
    assert_eq!(result.execution, Execution::BudgetExhausted);
    assert_eq!(result.snapshot_comparison_bytes, MAX_SNAPSHOT_BYTES);
    assert_eq!(result.unavailable_plans.len(), 20);
    assert_eq!(result.overlap_groups, [vec![100, 101]]);
}

#[test]
fn variable_intent_footprints_compare_every_required_region() {
    let plans = [
        Plan {
            id: 0,
            required_regions: 1,
            regions: vec![region("shared.rs", 10, 20)],
        },
        Plan {
            id: 1,
            required_regions: 3,
            regions: vec![
                region("a.rs", 0, 10),
                region("b.rs", 0, 10),
                region("shared.rs", 12, 18),
            ],
        },
    ];
    let result = analyze(&plans, true);
    assert!(result.coverage_complete);
    assert_eq!(result.region_comparisons, 3);
    assert_eq!(
        result.relations[0].relation,
        Relation::OverlapWitness {
            witnesses: vec![Witness {
                first_region: 0,
                second_region: 2,
                intersection: ByteSpan { start: 12, end: 18 },
            }],
        }
    );
}

#[test]
fn later_regions_cannot_hide_snapshot_contradictions() {
    let mut changed = region("shared.rs", 30, 40);
    changed.snapshot = "different exact snapshot";
    let plans = [Plan {
        id: 7,
        required_regions: 3,
        regions: vec![region("shared.rs", 0, 10), region("a.rs", 0, 10), changed],
    }];
    let result = analyze(&plans, true);
    assert!(!result.coverage_complete);
    assert_eq!(result.conflicting_snapshot_plans, [7]);
}

#[test]
fn declared_footprints_require_every_region_and_nonzero_cardinality() {
    for required_regions in [0, 3] {
        let plans = [Plan {
            id: 3,
            required_regions,
            regions: vec![region("a.rs", 0, 10)],
        }];
        let result = analyze(&plans, true);
        assert!(!result.coverage_complete);
        assert_eq!(result.execution, Execution::Completed);
        assert_eq!(result.unavailable_plans, [3]);
    }
}

#[test]
fn region_visits_are_bounded_even_for_one_large_intent() {
    let plans = [Plan {
        id: 9,
        required_regions: MAX_COMPARISONS + 1,
        regions: (0..MAX_COMPARISONS + 1)
            .map(|_| region("shared.rs", 0, 10))
            .collect(),
    }];
    let result = analyze(&plans, true);
    assert_eq!(result.execution, Execution::BudgetExhausted);
    assert!(!result.coverage_complete);
    assert_eq!(result.unavailable_plans, [9]);
    assert_eq!(result.expected_comparisons, Some(0));
    assert_eq!(result.assessed_comparisons, 0);
}
