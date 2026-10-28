//! Retained choices use shared owner resolution and rule-specific measurements.
use super::super::{
    Locator, ReviewStore, Versions, exact_rules, length_rules, local, local_adapter,
    owner_population::{Domain, Population},
    source_rules, spacing_rules, status,
};
use super::observed::{Observations, Row, Rule};
use crate::clones::report;

type Assessment = (status::State, String);
#[derive(Clone, Copy)]
enum Entry<'a> {
    Parameter(&'a source_rules::Entry),
    Length(&'a length_rules::Entry),
    Spacing(&'a spacing_rules::Entry),
}

pub(super) fn records(
    store: &ReviewStore,
    observations: &Observations,
    ctx: &report::Context<'_>,
) -> Vec<status::Record> {
    if !ctx.args.review_status_requested() {
        return Vec::new();
    }
    let population = Population::new(
        &observations.rows,
        observations.unavailable_scope,
        observations.truncated,
    );
    let parameters = store
        .parameter_entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (index, Entry::Parameter(entry)));
    let lengths = store
        .length_entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (index, Entry::Length(entry)));
    let spacings = store
        .spacing_entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (index, Entry::Spacing(entry)));

    parameters
        .chain(lengths)
        .chain(spacings)
        .map(|(index, entry)| {
            let assessment = if ctx.args.source_rules && entry.requested(observations) {
                population
                    .resolve(entry.identity().0, entry.identity().1, Domain::Source)
                    .map_or_else(|assessment| assessment, |owner| entry.assess_owner(owner))
            } else {
                (
                    status::State::NotRequested,
                    "Source rules were not requested".into(),
                )
            };
            entry.record(index, assessment)
        })
        .collect()
}

impl<'a> Entry<'a> {
    fn identity(self) -> (&'a Locator, &'a local::BuildContext) {
        match self {
            Self::Parameter(entry) => (&entry.finding.locator, &entry.finding.context),
            Self::Length(entry) => (&entry.finding.locator, &entry.finding.context),
            Self::Spacing(entry) => (&entry.finding.locator, &entry.finding.context),
        }
    }

    fn record(self, entry_index: usize, (state, detail): Assessment) -> status::Record {
        let (family, reason) = match self {
            Self::Parameter(entry) => (status::Family::Parameter, &entry.reason),
            Self::Length(entry) => (status::Family::Length, &entry.reason),
            Self::Spacing(entry) => (status::Family::Spacing, &entry.reason),
        };
        status::Record {
            family,
            entry_index,
            state,
            detail,
            reason: reason.clone(),
            members: vec![self.identity().0.clone()],
        }
    }

    fn assess_owner(self, row: &Row) -> Assessment {
        match self {
            Self::Parameter(entry) => parameter_owner(row, &entry.finding),
            Self::Length(entry) => length_owner(row, &entry.finding),
            Self::Spacing(entry) => spacing_owner(row, &entry.finding),
        }
    }

    fn requested(self, observations: &Observations) -> bool {
        !matches!(self, Self::Spacing(_)) || observations.spacing_requested
    }
}

fn parameter_owner(owner: &Row, previous: &source_rules::Snapshot) -> Assessment {
    if let Some(assessment) = gate(
        &owner.parameters,
        &previous.policy,
        &previous.versions,
        "fn_params",
    ) {
        return assessment;
    }
    assess_positive(
        owner.checked_parameter(),
        owner.parameter_snapshot(),
        previous,
    )
}

fn length_owner(owner: &Row, previous: &length_rules::Snapshot) -> Assessment {
    if let Some(assessment) = gate(
        &owner.lengths,
        &previous.policy,
        &previous.versions,
        "fn_length",
    ) {
        return assessment;
    }
    assess_positive(owner.checked_length(), owner.length_snapshot(), previous)
}

fn spacing_owner(owner: &Row, previous: &spacing_rules::Snapshot) -> Assessment {
    if !owner.spacing_requested {
        return (
            status::State::NotRequested,
            "Spacing source rule was not requested".into(),
        );
    }
    if let Some(assessment) = gate(
        &owner.spacings,
        &previous.policy,
        &previous.versions,
        "fn_spacing",
    ) {
        return assessment;
    }
    assess_positive(owner.checked_spacing(), owner.spacing_snapshot(), previous)
}

fn assess_positive<S: exact_rules::ReviewSnapshot>(
    checked: Result<exact_rules::LiveFinding<'_, S>, String>,
    snapshot: Result<&S, String>,
    previous: &S,
) -> Assessment {
    if let Err(reason) = checked {
        return unavailable(&reason);
    }
    let current = match snapshot {
        Ok(snapshot) => snapshot,
        Err(reason) => return unavailable(&reason),
    };
    positive(
        current == previous,
        exact_rules::stale_reason(previous, current),
    )
}

fn gate<P: PartialEq>(
    rule: &Rule<P>,
    previous: &P,
    versions: &Versions,
    detector: &str,
) -> Option<Assessment> {
    if !rule.complete {
        return Some(unavailable("Source rule assessment did not complete"));
    }
    (!rule.visible).then(|| negative(&rule.policy, previous, versions, detector))
}

fn positive(exact: bool, reason: Option<source_rules::StaleReason>) -> Assessment {
    if exact {
        (
            status::State::HiddenExact,
            "Exact source measurement and policy choice remains current".into(),
        )
    } else {
        (
            status::State::Stale,
            format!("Source finding changed: {reason:?}"),
        )
    }
}

fn unavailable(detail: &str) -> Assessment {
    (status::State::Unavailable, detail.into())
}

fn negative<P: PartialEq>(
    current: &P,
    previous: &P,
    versions: &Versions,
    detector: &str,
) -> Assessment {
    if versions != &local_adapter::versions(detector) {
        return unavailable(
            "Retained source detector versions cannot establish a current negative assessment",
        );
    }
    if current != previous {
        return (
            status::State::Stale,
            "Source rule policy changed; a completed negative does not retire the prior choice"
                .into(),
        );
    }
    (status::State::NoFinding, "Source rules completed under the retained policy without a visible finding, including source suppression".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_negative_requires_the_retained_parameter_policy() {
        let previous = source_rules::Policy {
            max_params: 4,
            count_self: false,
        };
        let versions = local_adapter::versions("fn_params");
        assert_eq!(
            negative(&previous, &previous, &versions, "fn_params").0,
            status::State::NoFinding
        );
        for current in [
            source_rules::Policy {
                max_params: 8,
                count_self: false,
            },
            source_rules::Policy {
                max_params: 4,
                count_self: true,
            },
        ] {
            assert_eq!(
                negative(&current, &previous, &versions, "fn_params").0,
                status::State::Stale
            );
        }
    }

    #[test]
    fn completed_negative_cannot_retire_a_different_detector_version() {
        let policy = source_rules::Policy {
            max_params: 4,
            count_self: false,
        };
        let mut versions = local_adapter::versions("fn_params");
        versions.detector_version += 1;
        assert_eq!(
            negative(&policy, &policy, &versions, "fn_params").0,
            status::State::Unavailable
        );
    }

    #[test]
    fn length_negative_preserves_changed_policy_and_unknown_detector_choices() {
        let previous = length_rules::Policy { max_lines: 50 };
        let raised = length_rules::Policy { max_lines: 100 };
        let versions = local_adapter::versions("fn_length");
        assert_eq!(
            negative(&previous, &previous, &versions, "fn_length").0,
            status::State::NoFinding
        );
        assert_eq!(
            negative(&raised, &previous, &versions, "fn_length").0,
            status::State::Stale
        );
        let mut future = versions;
        future.detector_version += 1;
        assert_eq!(
            negative(&previous, &previous, &future, "fn_length").0,
            status::State::Unavailable
        );
    }

    #[test]
    fn spacing_negative_preserves_disabled_raised_and_unknown_version_choices() {
        let previous = spacing_rules::Policy { min_statements: 5 };
        let versions = local_adapter::versions("fn_spacing");
        assert_eq!(
            negative(&previous, &previous, &versions, "fn_spacing").0,
            status::State::NoFinding
        );
        for current in [
            spacing_rules::Policy { min_statements: 0 },
            spacing_rules::Policy { min_statements: 10 },
        ] {
            assert_eq!(
                negative(&current, &previous, &versions, "fn_spacing").0,
                status::State::Stale
            );
        }
        let mut future = versions;
        future.detector_version += 1;
        assert_eq!(
            negative(&previous, &previous, &future, "fn_spacing").0,
            status::State::Unavailable
        );
    }
}
