//! Retained choices retire only after complete numeric negatives under the same policy.
use super::super::{
    ReviewStore, Versions, density_rules, exact_rules, local_adapter, numeric_workflow,
    owner_population::Domain, status,
};
use super::observed::{Observations, Row};
use crate::clones::report;

type Assessment = (status::State, String);

pub(super) fn records(
    store: &ReviewStore,
    observations: &Observations,
    ctx: &report::Context<'_>,
) -> Vec<status::Record> {
    numeric_workflow::records(&store.density_entries, observations, ctx)
}

impl numeric_workflow::Retained for Row {
    type Snapshot = density_rules::Snapshot;
    const DOMAIN: Domain = Domain::Density;
    const NOT_REQUESTED: &'static str = "Effect density was not requested";

    fn family(_snapshot: &Self::Snapshot) -> status::Family {
        status::Family::SideEffectDensity
    }

    fn assess(&self, previous: &Self::Snapshot) -> Assessment {
        assess_owner(self, previous)
    }
}

fn assess_owner(owner: &Row, previous: &density_rules::Snapshot) -> Assessment {
    if !owner.selected {
        return not_requested("Retained density owner is outside the current selected scope");
    }
    if !owner.complete {
        return unavailable("Density lacks a completed evaluated numeric assessment");
    }
    if !owner.visible {
        return match owner.policy() {
            Ok(policy) => negative(policy, &previous.policy, &previous.versions),
            Err(reason) => unavailable(reason),
        };
    }
    if let Err(reason) = owner.checked() {
        return unavailable(&reason);
    }

    let current = match owner.snapshot() {
        Ok(snapshot) => snapshot,
        Err(reason) => return unavailable(&reason),
    };
    if current == previous {
        (
            status::State::HiddenExact,
            "Exact density measurement and policy choice remains current".into(),
        )
    } else {
        (
            status::State::Stale,
            format!(
                "Density finding changed: {:?}",
                exact_rules::stale_reason(previous, current)
            ),
        )
    }
}

fn negative(
    current: &density_rules::Policy,
    previous: &density_rules::Policy,
    versions: &Versions,
) -> Assessment {
    if versions != &local_adapter::versions("side_effect_density") {
        return unavailable(
            "Retained density detector versions cannot establish a current negative assessment",
        );
    }
    if current != previous {
        return (
            status::State::Stale,
            "Density policy changed; a completed negative does not retire the prior choice".into(),
        );
    }
    (
        status::State::NoFinding,
        "Density completed under the retained policy with an evaluated numeric negative".into(),
    )
}

fn unavailable(detail: &str) -> Assessment {
    (status::State::Unavailable, detail.into())
}

fn not_requested(detail: &str) -> Assessment {
    (status::State::NotRequested, detail.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstyle_core::config::SideEffectsConfig;

    #[test]
    fn numeric_negative_requires_the_same_complete_policy_and_versions() {
        let config = SideEffectsConfig::default();
        let policy = density_rules::Policy::from_config(&config);
        let versions = local_adapter::versions("side_effect_density");
        assert_eq!(
            negative(&policy, &policy, &versions).0,
            status::State::NoFinding
        );
        let mut future = versions.clone();
        future.detector_version += 1;
        assert_eq!(
            negative(&policy, &policy, &future).0,
            status::State::Unavailable
        );

        let mut raised = config.clone();
        raised.max_density_milli += 1;
        let mut reweighted = config.clone();
        reweighted.weights.mutation += 1;
        let mut reclassified = config.clone();
        reclassified.paths.io.push("custom::write".into());
        let mut exempted = config;
        exempted.min_statements += 1;
        for changed in [raised, reweighted, reclassified, exempted] {
            assert_eq!(
                negative(
                    &density_rules::Policy::from_config(&changed),
                    &policy,
                    &versions
                )
                .0,
                status::State::Stale
            );
        }
        let mut metric = policy.clone();
        metric.event_metric.push_str("-future");
        assert_eq!(
            negative(&metric, &policy, &versions).0,
            status::State::Stale
        );
    }
}
