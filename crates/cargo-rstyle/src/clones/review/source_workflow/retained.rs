//! Retained parameter choices stay distinct from current finding presentation.
use super::super::{
    CompilationTarget, Locator, ReviewStore, Versions, local_adapter, source_rules, status,
};
use super::observed::{Observations, Row};
use crate::clones::report;
use std::collections::BTreeMap;

struct Population<'a> {
    owners: BTreeMap<(&'a Locator, &'a CompilationTarget), Vec<&'a Row>>,
    uncertain: bool,
    truncated: bool,
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
    store
        .parameter_entries
        .iter()
        .enumerate()
        .map(|(entry_index, entry)| {
            let (state, detail) = if ctx.args.source_rules {
                population.assess(entry)
            } else {
                (
                    status::State::NotRequested,
                    "Parameter source rules were not requested".into(),
                )
            };
            status::Record {
                family: status::Family::Parameter,
                entry_index,
                state,
                detail,
                reason: entry.reason.clone(),
                members: vec![entry.finding.locator.clone()],
            }
        })
        .collect()
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

    fn assess(&self, entry: &source_rules::Entry) -> (status::State, String) {
        if self.truncated {
            return unavailable("Parameter identity budget exhausted; uniqueness is unknown");
        }

        let snapshot = &entry.finding;
        let key = (&snapshot.locator, &snapshot.context.compilation_target);
        let Some(owners) = self.owners.get(&key) else {
            return self.missing();
        };
        if owners.len() != 1 {
            return unavailable("Parameter owner has ambiguous observations in this target");
        }
        let owner = owners[0];
        let Ok((_, context)) = owner.identity() else {
            return self.missing();
        };
        if context != &snapshot.context {
            return if self.uncertain {
                self.missing()
            } else {
                (
                    status::State::ContextChanged,
                    "Parameter owner was observed in a different build context".into(),
                )
            };
        }
        assess_owner(owner, snapshot)
    }

    fn missing(&self) -> (status::State, String) {
        if self.uncertain {
            unavailable(
                "Parameter identity scope is incomplete; a retained owner cannot be classified",
            )
        } else {
            (status::State::NotObserved, "Retained parameter owner was not observed; deletion and omitted capture scope are not distinguished".into())
        }
    }
}

fn assess_owner(owner: &Row, previous: &source_rules::Snapshot) -> (status::State, String) {
    if !owner.complete {
        return unavailable("Parameter source assessment did not complete");
    }
    if !owner.visible {
        let Some(policy) = owner.policy() else {
            return unavailable("Current parameter policy is unavailable");
        };
        return negative(policy, &previous.policy, &previous.versions);
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
            "Exact parameter signature and policy choice remains current".into(),
        )
    } else {
        (
            status::State::Stale,
            format!(
                "Parameter finding changed: {:?}",
                source_rules::stale_reason(previous, current)
            ),
        )
    }
}

fn unavailable(detail: &str) -> (status::State, String) {
    (status::State::Unavailable, detail.into())
}

fn negative(
    current: &source_rules::Policy,
    previous: &source_rules::Policy,
    versions: &Versions,
) -> (status::State, String) {
    if versions != &local_adapter::versions("fn_params") {
        return unavailable(
            "Retained parameter detector versions cannot establish a current negative assessment",
        );
    }
    if current != previous {
        return (
            status::State::Stale,
            "Parameter rule policy changed; a completed negative does not retire the prior choice"
                .into(),
        );
    }
    (
        status::State::NoFinding,
        "Parameter rules completed under the retained policy without a visible finding, including source suppression".into(),
    )
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
            negative(&previous, &previous, &versions).0,
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
                negative(&current, &previous, &versions).0,
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
            negative(&policy, &policy, &versions).0,
            status::State::Unavailable
        );
    }
}
