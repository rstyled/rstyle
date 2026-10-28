//! Retained mutation choices require current evaluated numeric observations.
use super::super::{
    ReviewStore, Versions, exact_rules, local_adapter, mutation_rules,
    owner_population::{Domain, Population},
    status,
};
use super::observed::{Observations, Row};
use crate::clones::report;

type Assessment = (status::State, String);

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
    store
        .mutation_entries
        .iter()
        .enumerate()
        .map(|(entry_index, entry)| {
            let (state, detail) = if observations.requested {
                population
                    .resolve(
                        &entry.finding.locator,
                        &entry.finding.context,
                        Domain::Mutation,
                    )
                    .map_or_else(
                        |assessment| assessment,
                        |owner| assess_owner(owner, &entry.finding),
                    )
            } else {
                not_requested("Mutation rules were not requested")
            };
            status::Record {
                family: family(&entry.finding.policy),
                entry_index,
                state,
                detail,
                reason: entry.reason.clone(),
                members: vec![entry.finding.locator.clone()],
            }
        })
        .collect()
}

fn family(policy: &mutation_rules::Policy) -> status::Family {
    match policy {
        mutation_rules::Policy::MutationPerTarget { .. } => status::Family::MutationPerTarget,
        mutation_rules::Policy::MutatedTargets { .. } => status::Family::MutatedTargets,
    }
}

fn assess_owner(owner: &Row, previous: &mutation_rules::Snapshot) -> Assessment {
    if !owner.selected {
        return not_requested("Retained mutation owner is outside the current selected scope");
    }
    let rule_id = previous.policy.rule_id();
    let rule = match owner.rule(rule_id) {
        Some(rule) => rule,
        None => return unavailable("Mutation rule was not assessed"),
    };
    if !rule.complete {
        return unavailable("Mutation rule lacks a completed evaluated numeric assessment");
    }
    if !rule.visible {
        return negative(&rule.policy, &previous.policy, &previous.versions);
    }

    if let Err(reason) = owner.checked(rule_id) {
        return unavailable(&reason);
    }
    let current = match owner.snapshot(rule_id) {
        Ok(snapshot) => snapshot,
        Err(reason) => return unavailable(&reason),
    };
    if current == previous {
        (
            status::State::HiddenExact,
            "Exact mutation measurement and policy choice remains current".into(),
        )
    } else {
        (
            status::State::Stale,
            format!(
                "Mutation finding changed: {:?}",
                exact_rules::stale_reason(previous, current)
            ),
        )
    }
}

fn negative(
    current: &mutation_rules::Policy,
    previous: &mutation_rules::Policy,
    versions: &Versions,
) -> Assessment {
    if versions != &local_adapter::versions(previous.rule_id()) {
        return unavailable(
            "Retained mutation detector versions cannot establish a current negative assessment",
        );
    }
    if current != previous {
        return (
            status::State::Stale,
            "Mutation rule policy changed; a completed negative does not retire the prior choice"
                .into(),
        );
    }
    (
        status::State::NoFinding,
        "Mutation rules completed under the retained policy with an evaluated numeric negative"
            .into(),
    )
}

fn unavailable(detail: &str) -> Assessment {
    (status::State::Unavailable, detail.into())
}

fn not_requested(detail: &str) -> Assessment {
    (status::State::NotRequested, detail.into())
}

#[cfg(test)]
mod tests;
