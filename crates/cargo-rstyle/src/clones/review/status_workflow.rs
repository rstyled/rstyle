//! Retained-entry assessment consumes completed findings and detector decisions.
use super::{Context, Locator, ReviewStore};
use super::{adapter, local, local_adapter, local_workflow, status, workflow};
use crate::clones::{FunctionKey, analysis, report};
use std::{cell::OnceCell, collections::BTreeMap};

struct Observation<C> {
    key: FunctionKey,
    context: C,
}

struct Observations<C> {
    members: BTreeMap<Locator, Vec<Observation<C>>>,
    failures: Vec<String>,
    truncated: bool,
}

impl<C: PartialEq> Observations<C> {
    fn resolve<'a>(
        &self,
        members: impl Iterator<Item = (&'a Locator, &'a C)>,
    ) -> Result<Vec<FunctionKey>, Assessment>
    where
        C: 'a,
    {
        let mut keys = Vec::new();
        let mut changed = None;
        for (locator, context) in members {
            match self.endpoint(locator, context) {
                Ok(key) => keys.push(key),
                Err(reason) if reason.state == status::State::ContextChanged => {
                    changed = Some(reason)
                }
                Err(reason) => return Err(reason),
            }
        }
        changed.map_or(Ok(keys), Err)
    }

    fn endpoint(&self, locator: &Locator, context: &C) -> Result<FunctionKey, Assessment> {
        if self.truncated {
            return Err(Assessment::new(
                status::State::Unavailable,
                "Retained-review observation budget exhausted; endpoint uniqueness is unknown",
            ));
        }
        let Some(observed) = self.members.get(locator) else {
            return Err(self.missing());
        };

        let mut matching = observed.iter().filter(|value| &value.context == context);
        let Some(first) = matching.next() else {
            return Err(if self.failures.is_empty() {
                Assessment::new(
                    status::State::ContextChanged,
                    "Endpoint observed only in a different build or detector context",
                )
            } else {
                self.missing()
            });
        };
        if matching.next().is_some() {
            return Err(Assessment::new(
                status::State::Unavailable,
                "Endpoint has more than one observation in the retained context",
            ));
        }
        Ok(first.key)
    }

    fn missing(&self) -> Assessment {
        if let Some(reason) = self.failures.first() {
            Assessment::new(
                status::State::Unavailable,
                format!(
                    "No complete checked endpoint identity; source or observation scope unavailable: {reason}"
                ),
            )
        } else {
            Assessment::new(
                status::State::NotObserved,
                "Endpoint not observed in this capture; deletion and omitted package/target/source scope are not distinguished",
            )
        }
    }
}

struct Assessment {
    state: status::State,
    detail: String,
}

impl Assessment {
    fn new(state: status::State, detail: impl Into<String>) -> Self {
        Self {
            state,
            detail: detail.into(),
        }
    }

    fn negative(completed: bool) -> Self {
        if completed {
            Self::new(
                status::State::NoFinding,
                "Retained endpoints were evaluated in their original context without this finding (including existing-trait suppression)",
            )
        } else {
            Self::new(
                status::State::Unavailable,
                "Relevant function or pair assessment did not complete; admission, unsupported evidence or analysis/pair budgets may exclude it",
            )
        }
    }
}

impl<C> Observations<C> {
    fn include(mut self, key: FunctionKey, identity: Result<(Locator, C), String>) -> Self {
        match identity {
            Ok((locator, context)) => self
                .members
                .entry(locator)
                .or_default()
                .push(Observation { key, context }),
            Err(reason) => self.failures.push(reason),
        }
        self
    }
}

fn observe<C>(
    ctx: &report::Context<'_>,
    mut identity: impl FnMut(&crate::clones::Function) -> Result<(Locator, C), String>,
) -> Observations<C> {
    let failures = ctx
        .capture
        .failures
        .iter()
        .chain(&ctx.handoff.failures)
        .cloned()
        .chain(
            (ctx.handoff.assessed.len() > 4096)
                .then(|| "Retained-review observation budget exhausted".into()),
        )
        .collect();
    let mut observed = Observations {
        members: BTreeMap::new(),
        failures,
        truncated: ctx.handoff.assessed.len() > 4096,
    };

    // Status has its own bounded identity work; it never enlarges detector budgets.
    for function in ctx.handoff.assessed.iter().take(4096) {
        observed = observed.include(function.key, identity(function));
    }
    observed
}

pub(super) fn helpers(
    entries: &ReviewStore,
    snapshots: &[workflow::Snapshot],
    helpers: &analysis::helper_reports::Report<'_>,
    ctx: &report::Context<'_>,
) -> Vec<status::Record> {
    if !ctx.args.review_status_requested() || entries.entries.is_empty() {
        return Vec::new();
    }
    let observed = OnceCell::new();
    let current: Vec<_> = snapshots
        .iter()
        .filter_map(|(_, _, value)| value.as_ref().ok())
        .filter(|snapshot| super::validate_snapshot(snapshot).is_ok())
        .collect();
    entries
        .entries
        .iter()
        .enumerate()
        .map(|(entry_index, entry)| {
            let assessment = helper_assessment(
                entry,
                ctx.args.helper_proposals.then_some(current.as_slice()),
                helpers,
                || {
                    observed.get_or_init(|| helper_observations(ctx)).resolve(
                        entry
                            .finding
                            .members
                            .iter()
                            .map(|member| (&member.key.locator, &member.key.context)),
                    )
                },
            );
            status::Record {
                family: status::Family::Helper,
                entry_index,
                state: assessment.state,
                detail: assessment.detail,
                reason: entry.reason.clone(),
                members: entry
                    .finding
                    .members
                    .iter()
                    .map(|member| member.key.locator.clone())
                    .collect(),
            }
        })
        .collect()
}

fn helper_observations(ctx: &report::Context<'_>) -> Observations<Context> {
    let policy = workflow::matching_policy(
        ctx.matching_config,
        crate::clones::matching_profile(ctx.args).identity(),
    );
    observe(ctx, |function| {
        let unit = workflow::unit(ctx.handoff, ctx.capture, function)?;
        let member = adapter::member_key(
            function.facts.full()?,
            unit,
            &ctx.capture.plan,
            adapter::Policies {
                matching: &policy,
                consumer: &workflow::consumer_policy(),
            },
        )?;
        Ok((member.locator, member.context))
    })
}

fn helper_assessment(
    entry: &super::ReviewEntry,
    snapshots: Option<&[&super::FindingSnapshot]>,
    helpers: &analysis::helper_reports::Report<'_>,
    observed: impl FnOnce() -> Result<Vec<FunctionKey>, Assessment>,
) -> Assessment {
    let Some(snapshots) = snapshots else {
        return Assessment::new(
            status::State::NotRequested,
            "Helper analysis was not requested",
        );
    };
    let current = || snapshots.iter().copied();

    if current().any(|snapshot| entry.finding == *snapshot) {
        return Assessment::new(status::State::HiddenExact, "Exact completed helper finding");
    }
    if current().any(|snapshot| super::permits_growth(entry, snapshot)) {
        return Assessment::new(
            status::State::HiddenGrowth,
            "Completed helper finding with permitted membership growth",
        );
    }
    if let Some(reason) =
        current().find_map(|snapshot| super::stale_reason(&entry.finding, snapshot))
    {
        return Assessment::new(
            status::State::Stale,
            format!("Current helper finding differs: {reason:?}"),
        );
    }
    if entry.finding.versions != workflow::versions() {
        return Assessment::new(
            status::State::Unavailable,
            "Retained helper detector/identity versions differ; no current finding establishes correspondence",
        );
    }
    match observed() {
        Err(assessment) => assessment,
        Ok(keys) => {
            let completed = match keys.as_slice() {
                [left, right] => {
                    helpers.review_pair_without_finding(&left.to_string(), &right.to_string())
                }
                _ => false,
            };
            Assessment::negative(completed)
        }
    }
}

pub(super) fn local_entries(
    store: &ReviewStore,
    snapshots: &[local_workflow::Snapshot],
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
) -> Vec<status::Record> {
    if !ctx.args.review_status_requested() || store.source_entries.is_empty() {
        return Vec::new();
    }
    let observed = OnceCell::new();
    let current: Vec<_> = snapshots
        .iter()
        .filter_map(|snapshot| snapshot.value.as_ref().ok())
        .filter(|snapshot| local::validate_snapshot(snapshot).is_ok())
        .collect();
    store
        .source_entries
        .iter()
        .enumerate()
        .map(|(entry_index, entry)| {
            let (family, requested) = match entry.finding.proposal {
                local::Proposal::EnumDispatch { .. } => {
                    (status::Family::EnumDispatch, ctx.args.enum_dispatch)
                }
                local::Proposal::OrderedCapability { .. } => {
                    (status::Family::CapabilitySet, ctx.args.capability_proposals)
                }
            };
            let assessment = local_assessment(
                entry,
                requested.then_some(current.as_slice()),
                || {
                    observed
                        .get_or_init(|| local_observations(inputs, ctx))
                        .resolve(
                            entry
                                .finding
                                .members
                                .iter()
                                .map(|member| (&member.locator, &member.context)),
                        )
                },
                inputs,
            );
            local_record(entry, entry_index, family, assessment)
        })
        .collect()
}

fn local_record(
    entry: &local::Entry,
    entry_index: usize,
    family: status::Family,
    assessment: Assessment,
) -> status::Record {
    status::Record {
        family,
        entry_index,
        state: assessment.state,
        detail: assessment.detail,
        reason: entry.reason.clone(),
        members: entry
            .finding
            .members
            .iter()
            .map(|member| member.locator.clone())
            .collect(),
    }
}

fn local_observations(
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
) -> Observations<local::BuildContext> {
    observe(ctx, |function| {
        let unit = workflow::unit(ctx.handoff, ctx.capture, function)?;
        local_adapter::member_key(local_adapter::Input {
            facts: function.facts.full()?,
            unit,
            plan: &ctx.capture.plan,
            index: inputs.indices.get(function.key.unit())?,
        })
    })
}

fn local_assessment(
    entry: &local::Entry,
    snapshots: Option<&[&local::Snapshot]>,
    observed: impl FnOnce() -> Result<Vec<FunctionKey>, Assessment>,
    inputs: &workflow::Inputs<'_, '_>,
) -> Assessment {
    let Some(snapshots) = snapshots else {
        return Assessment::new(
            status::State::NotRequested,
            "This source-local analysis family was not requested",
        );
    };
    let current = || snapshots.iter().copied();

    if current().any(|snapshot| entry.finding == *snapshot) {
        return Assessment::new(
            status::State::HiddenExact,
            "Exact completed source-local finding",
        );
    }
    if let Some(reason) =
        current().find_map(|snapshot| local::stale_reason(&entry.finding, snapshot))
    {
        return Assessment::new(
            status::State::Stale,
            format!("Current source-local finding differs: {reason:?}"),
        );
    }
    if entry.finding.versions != source_versions(&entry.finding.proposal) {
        return Assessment::new(
            status::State::Unavailable,
            "Retained source-local detector/identity versions differ; no current finding establishes correspondence",
        );
    }
    match observed() {
        Err(assessment) => assessment,
        Ok(keys) => {
            let completed = match keys.as_slice() {
                [key] => inputs.dispatch.review_without_finding(*key),
                [left, right] => inputs
                    .capabilities
                    .review_pair_without_finding(*left, *right),
                _ => false,
            };
            Assessment::negative(completed)
        }
    }
}

fn source_versions(proposal: &local::Proposal) -> super::Versions {
    let detector = match proposal {
        local::Proposal::EnumDispatch { .. } => "local-enum-dispatch",
        local::Proposal::OrderedCapability { .. } => "ordered-local-capability",
    };
    local_adapter::versions(detector)
}

#[cfg(test)]
mod tests;
