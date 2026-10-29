use super::super::Summary;

fn completed(functions: usize, density_sum: usize) -> Summary {
    Summary {
        functions,
        mean_density_milli: density_sum / functions.max(1),
        density_sum_milli: Some(density_sum),
        ..Summary::default()
    }
}

#[test]
fn aggregation_preserves_rounding_remainders_and_completed_counts() {
    let a = Summary {
        violations: 2,
        complexity_measured: 1,
        complexity_unavailable: 1,
        max_density_milli: 6,
        effect_counts: [("io".into(), 2)].into(),
        ..completed(2, 11)
    };
    let b = Summary {
        violations: 1,
        complexity_measured: 1,
        max_density_milli: 8,
        effect_counts: [("io".into(), 3), ("unknown".into(), 1)].into(),
        ..completed(1, 8)
    };
    let total = Summary::aggregate([&a, &b]).unwrap();
    assert_eq!(total.functions, 3);
    assert_eq!(total.mean_density_milli, 6);
    assert_eq!(total.density_sum_milli, Some(19));
    assert_eq!(total.max_density_milli, 8);
    assert_eq!(total.violations, 3);
    assert_eq!(total.complexity_measured, 2);
    assert_eq!(total.complexity_unavailable, 1);
    assert_eq!(
        total.effect_counts,
        [("io".into(), 5), ("unknown".into(), 1)].into()
    );
}

#[test]
fn aggregation_distinguishes_absent_inputs_from_completed_empty_scope() {
    assert_eq!(Summary::aggregate([]), None);
    assert_eq!(Summary::aggregate([&Summary::default()]), None);
    assert_eq!(
        Summary::aggregate([&completed(0, 0)]),
        Some(completed(0, 0))
    );
    assert_eq!(
        Summary::aggregate([&completed(1, 2), &Summary::default()]),
        None
    );
}

#[test]
fn aggregation_rejects_density_and_count_overflow() {
    let maximum = completed(1, usize::MAX);
    assert_eq!(Summary::aggregate([&maximum, &completed(1, 1)]), None);
    let overflow_cases = [
        Summary {
            functions: usize::MAX,
            ..completed(0, 0)
        },
        Summary {
            violations: usize::MAX,
            ..completed(0, 0)
        },
        Summary {
            complexity_measured: usize::MAX,
            ..completed(0, 0)
        },
        Summary {
            complexity_unavailable: usize::MAX,
            ..completed(0, 0)
        },
        Summary {
            effect_counts: [("io".into(), usize::MAX)].into(),
            ..completed(0, 0)
        },
    ];
    let increment = Summary {
        violations: 1,
        complexity_measured: 1,
        complexity_unavailable: 1,
        effect_counts: [("io".into(), 1)].into(),
        ..completed(1, 0)
    };
    for maximum in &overflow_cases {
        assert_eq!(Summary::aggregate([maximum, &increment]), None);
    }
}

#[test]
fn retained_density_total_does_not_change_serialized_summary() {
    let summary = completed(2, 11);
    let mut without_total = summary.clone();
    without_total.density_sum_milli = None;
    let value = serde_json::to_value(&summary).unwrap();
    assert_eq!(value, serde_json::to_value(without_total).unwrap());
    assert_eq!(value.as_object().unwrap().len(), 7);
}
