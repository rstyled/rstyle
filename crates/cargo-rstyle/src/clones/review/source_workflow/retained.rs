//! Retained choices use shared owner resolution and rule-specific measurements.
use super::super::{
    CompilationTarget, Locator, ReviewStore, Versions, length_rules, local, local_adapter,
    source_rules, status,
};
use super::observed::{Observations, Row, Rule};
use crate::clones::report;
use std::collections::BTreeMap;

type Assessment = (status::State, String);
struct Population<'a> {
    owners: BTreeMap<(&'a Locator, &'a CompilationTarget), Vec<&'a Row>>,
    uncertain: bool,
    truncated: bool,
}

#[derive(Clone, Copy)]
enum Entry<'a> {
    Parameter(&'a source_rules::Entry),
    Length(&'a length_rules::Entry),
}

pub(super) fn records(
    store: &ReviewStore,
    observations: &Observations,
    ctx: &report::Context<'_>,
) -> Vec<status::Record> {
    if !ctx.args.review_status_requested() {
        return Vec::new();
    }
    let population = Population::new(observations);
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

    parameters
        .chain(lengths)
        .map(|(index, entry)| {
            let assessment = if ctx.args.source_rules {
                population.assess(entry)
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
        }
    }

    fn record(self, entry_index: usize, (state, detail): Assessment) -> status::Record {
        let (family, reason) = match self {
            Self::Parameter(entry) => (status::Family::Parameter, &entry.reason),
            Self::Length(entry) => (status::Family::Length, &entry.reason),
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
        }
    }
}

impl<'a> Population<'a> {
    fn new(observations: &'a Observations) -> Self {
        let mut owners = BTreeMap::<_, Vec<_>>::new();
        for row in &observations.rows {
            if let Ok((locator, context)) = row.identity() {
                owners
                    .entry((locator, &context.compilation_target))
                    .or_default()
                    .push(row);
            }
        }
        Self {
            owners,
            uncertain: observations.unavailable_scope
                || observations.rows.iter().any(|row| row.identity().is_err()),
            truncated: observations.truncated,
        }
    }

    fn assess(&self, entry: Entry<'_>) -> Assessment {
        if self.truncated {
            return unavailable("Source identity budget exhausted; uniqueness is unknown");
        }
        let (locator, context) = entry.identity();
        let Some(owners) = self.owners.get(&(locator, &context.compilation_target)) else {
            return self.missing();
        };
        if owners.len() != 1 {
            return unavailable("Source owner has ambiguous observations in this target");
        }
        let owner = owners[0];
        let Ok((_, observed)) = owner.identity() else {
            return self.missing();
        };

        if context != observed {
            return if self.uncertain {
                self.missing()
            } else {
                (
                    status::State::ContextChanged,
                    "Source owner was observed in a different build context".into(),
                )
            };
        }
        entry.assess_owner(owner)
    }

    fn missing(&self) -> Assessment {
        if self.uncertain {
            unavailable(
                "Source identity scope is incomplete; a retained owner cannot be classified",
            )
        } else {
            (status::State::NotObserved, "Retained source owner was not observed; deletion and omitted capture scope are not distinguished".into())
        }
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
    if let Err(reason) = owner.checked_parameter() {
        return unavailable(&reason);
    }
    let current = match owner.parameter_snapshot() {
        Ok(snapshot) => snapshot,
        Err(reason) => return unavailable(&reason),
    };
    positive(
        current == previous,
        source_rules::stale_reason(previous, current),
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
    if let Err(reason) = owner.checked_length() {
        return unavailable(&reason);
    }
    let current = match owner.length_snapshot() {
        Ok(snapshot) => snapshot,
        Err(reason) => return unavailable(&reason),
    };
    positive(
        current == previous,
        length_rules::stale_reason(previous, current),
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
}
