use super::*;

#[test]
fn numeric_negative_requires_the_same_rule_policy_and_detector_versions() {
    let per_target = mutation_rules::Policy::MutationPerTarget {
        max_mutations_per_target: 2,
    };
    let targets = mutation_rules::Policy::MutatedTargets {
        max_mutated_targets: 2,
    };
    for policy in [&per_target, &targets] {
        let versions = local_adapter::versions(policy.rule_id());
        assert_eq!(
            negative(policy, policy, &versions).0,
            status::State::NoFinding
        );
        let mut future = versions;
        future.detector_version += 1;
        assert_eq!(
            negative(policy, policy, &future).0,
            status::State::Unavailable
        );
    }
    assert_eq!(
        negative(
            &targets,
            &per_target,
            &local_adapter::versions(per_target.rule_id())
        )
        .0,
        status::State::Stale
    );
}

#[test]
fn raised_limits_cannot_retire_prior_numeric_choices() {
    for (previous, raised) in [
        (
            mutation_rules::Policy::MutationPerTarget {
                max_mutations_per_target: 2,
            },
            mutation_rules::Policy::MutationPerTarget {
                max_mutations_per_target: 10,
            },
        ),
        (
            mutation_rules::Policy::MutatedTargets {
                max_mutated_targets: 2,
            },
            mutation_rules::Policy::MutatedTargets {
                max_mutated_targets: 10,
            },
        ),
    ] {
        assert_eq!(
            negative(
                &raised,
                &previous,
                &local_adapter::versions(previous.rule_id())
            )
            .0,
            status::State::Stale
        );
    }
}
