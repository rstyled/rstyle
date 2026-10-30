use super::*;
use rstyle_pdg::ByteSpan;

fn plan(id: usize, start: u32, end: u32) -> model::Plan<'static> {
    model::Plan {
        id,
        required_regions: 1,
        regions: vec![model::Region {
            resource: "src/lib.rs",
            snapshot: "retained original source",
            span: ByteSpan { start, end },
        }],
    }
}

fn three_families() -> Population<'static> {
    let mut population = Population::default();
    population = population.append(
        Family::Helper,
        Status {
            execution: Execution::Completed,
            complete: true,
        },
        [plan(0, 0, 12)].into_iter(),
    );
    population = population.append(
        Family::Parameter,
        Status {
            execution: Execution::Completed,
            complete: true,
        },
        [plan(0, 3, 7)].into_iter(),
    );
    population = population.append(
        Family::Guard,
        Status {
            execution: Execution::Completed,
            complete: true,
        },
        [plan(0, 12, 20)].into_iter(),
    );

    population
}

#[test]
fn equal_local_ids_stay_distinct_and_cross_family_regions_coordinate() {
    let population = three_families();
    let report = population.finish();
    assert!(report.permits_success());
    assert_eq!(
        report
            .plans
            .iter()
            .map(|reference| reference.id)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(
        report
            .plans
            .iter()
            .map(|reference| reference.local_id)
            .collect::<Vec<_>>(),
        [0, 0, 0]
    );
    assert_eq!(
        report
            .plans
            .iter()
            .map(|reference| reference.family)
            .collect::<Vec<_>>(),
        [Family::Helper, Family::Parameter, Family::Guard]
    );
    assert_eq!(report.coordination.overlap_groups, [vec![0, 1]]);
}

#[test]
fn complete_empty_family_is_clean_but_incomplete_empty_family_is_not() {
    let mut complete = Population::default();
    complete = complete.append(
        Family::Guard,
        Status {
            execution: Execution::Completed,
            complete: true,
        },
        std::iter::empty(),
    );
    let complete = complete.finish();
    assert!(complete.permits_success());
    assert_eq!(complete.expected_plans, Some(0));

    let mut incomplete = Population::default();
    incomplete = incomplete.append(
        Family::Guard,
        Status {
            execution: Execution::Completed,
            complete: false,
        },
        std::iter::empty(),
    );
    let incomplete = incomplete.finish();
    assert!(!incomplete.permits_success());
    assert_eq!(incomplete.expected_plans, None);
}

#[test]
fn union_plan_cap_preserves_references_and_marks_incomplete_execution() {
    let mut population = Population::default();
    population = population.append(
        Family::Helper,
        Status {
            execution: Execution::Completed,
            complete: true,
        },
        (0..model::MAX_PLANS).map(|id| model::Plan {
            id,
            required_regions: 1,
            regions: vec![],
        }),
    );
    population = population.append(
        Family::Guard,
        Status {
            execution: Execution::Completed,
            complete: true,
        },
        [plan(0, 0, 1)].into_iter(),
    );

    let report = population.finish();
    assert_eq!(report.plans.len(), model::MAX_PLANS);
    assert_eq!(report.expected_plans, Some(model::MAX_PLANS + 1));
    assert_eq!(report.execution, Execution::BudgetExhausted);
    assert!(!report.permits_success());
}

#[test]
fn failed_family_precedes_union_and_input_budget_exhaustion() {
    let mut population = Population::default();
    population = population.append(
        Family::Parameter,
        Status {
            execution: Execution::BudgetExhausted,
            complete: false,
        },
        std::iter::empty(),
    );
    population = population.append(
        Family::Guard,
        Status {
            execution: Execution::Failed,
            complete: false,
        },
        std::iter::empty(),
    );
    let report = population.finish();
    assert_eq!(report.execution, Execution::Failed);
    assert!(!report.permits_success());
    assert_eq!(report.expected_plans, None);
}

#[test]
fn missing_required_region_cannot_prove_a_disjoint_complete_plan() {
    let mut population = Population::default();
    population = population.append(
        Family::Guard,
        Status {
            execution: Execution::Completed,
            complete: true,
        },
        [model::Plan {
            id: 8,
            required_regions: 1,
            regions: vec![],
        }]
        .into_iter(),
    );
    let report = population.finish();
    assert!(!report.permits_success());
    assert_eq!(report.coordination.unavailable_plans, [0]);
}

#[test]
fn helper_replacement_overlaps_guard_with_the_same_original_body() {
    let mut population = Population::default();
    population = population.append(
        Family::Helper,
        Status {
            execution: Execution::Completed,
            complete: true,
        },
        [plan(4, 0, 20)].into_iter(),
    );
    population = population.append(
        Family::Guard,
        Status {
            execution: Execution::Completed,
            complete: true,
        },
        [plan(4, 5, 20)].into_iter(),
    );
    let report = population.finish();

    assert!(report.permits_success());
    assert_eq!(report.coordination.overlap_groups, [vec![0, 1]]);
    assert!(matches!(
        report.coordination.relations[0].relation,
        model::Relation::OverlapWitness { .. }
    ));
}

#[test]
fn conflicting_family_snapshots_never_prove_independence() {
    let mut guard = plan(0, 12, 20);
    guard.regions[0].snapshot = "different original bytes";
    let mut population = Population::default();
    population = population.append(
        Family::Helper,
        Status {
            execution: Execution::Completed,
            complete: true,
        },
        [plan(0, 0, 12)].into_iter(),
    );
    population = population.append(
        Family::Guard,
        Status {
            execution: Execution::Completed,
            complete: true,
        },
        [guard].into_iter(),
    );

    let report = population.finish();
    assert!(!report.permits_success());
    assert_eq!(report.coordination.conflicting_snapshot_plans, [0, 1]);
    assert!(matches!(
        report.coordination.relations[0].relation,
        model::Relation::ConflictingSnapshots
    ));
}

#[test]
fn cross_family_pairs_share_one_comparison_budget() {
    let mut population = Population::default();
    population = population.append(
        Family::Helper,
        Status {
            execution: Execution::Completed,
            complete: true,
        },
        (0..182).map(|id| plan(id, 0, 1)),
    );
    population = population.append(
        Family::Guard,
        Status {
            execution: Execution::Completed,
            complete: true,
        },
        (0..182).map(|id| plan(id, 0, 1)),
    );
    let report = population.finish();

    assert_eq!(report.expected_plans, Some(364));
    assert_eq!(report.execution, Execution::BudgetExhausted);
    assert!(report.coordination.assessed_comparisons <= model::MAX_COMPARISONS);
    assert!(!report.permits_success());
}
