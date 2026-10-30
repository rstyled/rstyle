//! Explicit source-proposal review consumes completed findings, never grants evidence.
use super::{
    ReviewStore, local, local_adapter, outcome, status, status_workflow, summary, workflow,
};
use crate::clones::analysis::Completion;
use crate::clones::{Function, FunctionKey, analysis, report};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Family {
    EnumDispatch,
    CapabilitySet,
}

#[derive(Serialize)]
pub(in crate::clones) struct Report {
    #[serde(flatten)]
    summary: summary::Summary,
    dispatch_visible_findings: usize,
    capability_visible_findings: usize,
    decisions: Vec<Decision>,
}

#[derive(Serialize)]
struct Decision {
    family: Family,
    endpoints: Vec<String>,
    #[serde(flatten)]
    outcome: outcome::Outcome,
}

pub(super) struct Snapshot {
    family: Family,
    endpoints: Vec<String>,
    pub value: Result<local::Snapshot, String>,
}

pub(super) fn apply(
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
    store: Option<&mut ReviewStore>,
) -> Result<(Report, Vec<status::Record>), String> {
    let requested = ctx.args.enum_dispatch || ctx.args.capability_proposals;
    let Some(store) = store.filter(|_| requested || ctx.args.review_status_requested()) else {
        return Ok((summarize(Vec::new(), 0, false, false), Vec::new()));
    };

    let snapshots = snapshots(inputs, ctx);
    dismiss(store, &snapshots, ctx)?;
    let checked = local::checked(&store.source_entries).map_err(|error| format!("{error:?}"))?;
    let decisions = snapshots
        .iter()
        .map(|snapshot| decision(snapshot, &checked))
        .collect();

    let records = status_workflow::local_entries(store, &snapshots, inputs, ctx);
    Ok((
        summarize(
            decisions,
            store.source_entries.len(),
            requested,
            inputs.dispatch.permits_success() && inputs.capabilities.permits_success(),
        ),
        records,
    ))
}

fn snapshots(inputs: &workflow::Inputs<'_, '_>, ctx: &report::Context<'_>) -> Vec<Snapshot> {
    let records = ctx
        .handoff
        .assessed
        .iter()
        .map(|function| (function.key, function))
        .collect();
    let mut resolver = Resolver {
        inputs,
        ctx,
        records,
        endpoints: BTreeMap::new(),
    };
    let mut snapshots: Vec<_> = inputs
        .dispatch
        .finding_proposals()
        .map(|(key, proposal)| {
            let value = resolver.dispatch(key, proposal);
            Snapshot {
                family: Family::EnumDispatch,
                endpoints: vec![key.to_string()],
                value,
            }
        })
        .collect();

    snapshots.extend(inputs.capabilities.finding_proposals().map(|finding| {
        let value = resolver.capability(&finding);
        Snapshot {
            family: Family::CapabilitySet,
            endpoints: vec![finding.left.to_string(), finding.right.to_string()],
            value,
        }
    }));
    snapshots
}

struct Resolver<'a, 'b, 'c> {
    inputs: &'a workflow::Inputs<'b, 'c>,
    ctx: &'a report::Context<'c>,
    records: BTreeMap<FunctionKey, &'c Function>,
    endpoints: BTreeMap<FunctionKey, Result<local_adapter::Endpoint, String>>,
}

impl Resolver<'_, '_, '_> {
    fn dispatch(
        &self,
        key: FunctionKey,
        proposal: &rstyle_pdg::consumer_facts::dispatch::Candidate,
    ) -> Result<local::Snapshot, String> {
        let record = self
            .records
            .get(&key)
            .ok_or("Missing dispatch review owner")?;
        let unit = workflow::unit(self.ctx.handoff, self.ctx.capture, record)?;
        local_adapter::dispatch(
            local_adapter::Input {
                facts: record.facts.full()?,
                unit,
                plan: &self.ctx.capture.plan,
                index: self.inputs.indices.get(key.unit())?,
            },
            proposal,
        )
    }

    fn endpoint(
        &mut self,
        key: FunctionKey,
        wrapper: &rstyle_pdg::consumer_facts::dispatch::capabilities::Wrapper,
    ) -> Result<&local_adapter::Endpoint, String> {
        let value = self.endpoints.entry(key).or_insert_with(|| {
            let record = self
                .records
                .get(&key)
                .ok_or("Missing capability review owner")?;
            let unit = workflow::unit(self.ctx.handoff, self.ctx.capture, record)?;
            local_adapter::capability_member(
                local_adapter::Input {
                    facts: record.facts.full()?,
                    unit,
                    plan: &self.ctx.capture.plan,
                    index: self.inputs.indices.get(key.unit())?,
                },
                wrapper,
            )
        });
        value.as_ref().map_err(Clone::clone)
    }

    fn capability(
        &mut self,
        finding: &analysis::capability_reports::Finding<'_>,
    ) -> Result<local::Snapshot, String> {
        self.endpoint(finding.left, finding.first)?;
        self.endpoint(finding.right, finding.second)?;
        local_adapter::capability(
            self.endpoints[&finding.left]
                .as_ref()
                .map_err(Clone::clone)?,
            self.endpoints[&finding.right]
                .as_ref()
                .map_err(Clone::clone)?,
        )
    }
}

fn dismiss(
    store: &mut ReviewStore,
    snapshots: &[Snapshot],
    ctx: &report::Context<'_>,
) -> Result<(), String> {
    let selection = if let Some(key) = &ctx.args.dismiss_enum_dispatch {
        Some((Family::EnumDispatch, vec![key.as_str()]))
    } else if !ctx.args.dismiss_capability_pair.is_empty() {
        Some((
            Family::CapabilitySet,
            ctx.args
                .dismiss_capability_pair
                .iter()
                .map(String::as_str)
                .collect(),
        ))
    } else {
        None
    };
    let Some((family, endpoints)) = selection else {
        return Ok(());
    };

    let snapshot = snapshots
        .iter()
        .find(|snapshot| {
            snapshot.family == family
                && snapshot.endpoints.len() == endpoints.len()
                && snapshot
                    .endpoints
                    .iter()
                    .all(|key| endpoints.contains(&key.as_str()))
        })
        .ok_or("Selected saved-run endpoints do not form the requested abstraction finding")?;
    let reason = ctx
        .args
        .review_reason
        .as_deref()
        .filter(|reason| !reason.trim().is_empty())
        .ok_or("A dismissal requires a nonempty review reason")?;

    let value = snapshot.value.as_ref().map_err(Clone::clone)?;
    let current = local::LiveFinding::checked(value).map_err(|error| format!("{error:?}"))?;
    local::record_dismissal(&mut store.source_entries, &current, reason.into())
        .map_err(|error| format!("{error:?}"))?;
    store.schema_version = store.schema_version.max(2);
    Ok(())
}

fn decision(snapshot: &Snapshot, store: &local::ValidatedStore<'_>) -> Decision {
    let current = snapshot
        .value
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|value| local::LiveFinding::checked(value).map_err(|error| format!("{error:?}")));
    let outcome = outcome::Outcome::checked(current.map(|current| store.lookup(&current).into()));
    Decision {
        family: snapshot.family,
        endpoints: snapshot.endpoints.clone(),
        outcome,
    }
}

fn summarize(
    decisions: Vec<Decision>,
    retained_entries: usize,
    requested: bool,
    scope_complete: bool,
) -> Report {
    let visible = |family| {
        decisions
            .iter()
            .filter(|decision| {
                decision.family == family && decision.outcome.state != "hidden_exact"
            })
            .count()
    };
    let summary = summary::summarize(
        &decisions,
        retained_entries,
        summary::Scope {
            requested,
            complete: scope_complete,
        },
        summary::Contract {
            schema: "rstyle.abstraction-review/1",
            incomplete_scope: "Potential abstraction findings remain unavailable; see enum_dispatch and capability_proposals coverage",
        },
    );
    Report {
        summary,
        dispatch_visible_findings: visible(Family::EnumDispatch),
        capability_visible_findings: visible(Family::CapabilitySet),
        decisions,
    }
}

impl summary::Decision for Decision {
    fn state(&self) -> &'static str {
        self.outcome.state
    }
}

impl Report {
    pub(in crate::clones) fn dispatch_visible_or(&self, unfiltered: usize) -> usize {
        self.summary
            .visible_or(self.dispatch_visible_findings, unfiltered)
    }

    pub(in crate::clones) fn capability_visible_or(&self, unfiltered: usize) -> usize {
        self.summary
            .visible_or(self.capability_visible_findings, unfiltered)
    }

    pub(in crate::clones) fn render(&self) -> String {
        if !self.summary.requested() {
            return String::new();
        }
        let mut output = format!(
            "\nAbstraction reviews: {:?} coverage; {} visible, {} hidden; {} retained entries.",
            self.summary.coverage,
            self.summary.visible_findings,
            self.summary.hidden_findings,
            self.summary.retained_entries
        );
        for decision in &self.decisions {
            output.push_str(&format!(
                "\nReview {}: {}. {}",
                decision.endpoints.join(" <> "),
                decision.outcome.state,
                decision.outcome.reason.as_deref().unwrap_or("")
            ));
        }
        output
    }
}

impl crate::clones::analysis::Completion for Report {
    fn permits_success(&self) -> bool {
        self.summary.permits_success()
    }
}
