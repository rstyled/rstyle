//! Shared lifecycle for numeric reviews over independently completed observations.
use super::{
    exact_rules, owner_observations,
    owner_population::{self, Domain, Population},
    status, summary,
};
use crate::compiler::report;

/// Each numeric family supplies its own current evidence assessment.
pub(super) trait Retained: owner_population::Owner {
    type Snapshot: exact_rules::ReviewSnapshot;
    const DOMAIN: Domain;
    const NOT_REQUESTED: &'static str;

    fn family(snapshot: &Self::Snapshot) -> status::Family;
    fn assess(&self, previous: &Self::Snapshot) -> owner_population::Assessment;
}

pub(super) fn records<R: Retained>(
    entries: &[exact_rules::Entry<R::Snapshot>],
    observations: &owner_observations::Observations<R>,
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
    entries
        .iter()
        .enumerate()
        .map(|(entry_index, entry)| {
            let (locator, context) = exact_rules::ReviewSnapshot::identity(&entry.finding);
            let (state, detail) = if observations.requested {
                population.resolve(locator, context, R::DOMAIN).map_or_else(
                    |assessment| assessment,
                    |owner| owner.assess(&entry.finding),
                )
            } else {
                (status::State::NotRequested, R::NOT_REQUESTED.into())
            };
            status::Record {
                family: R::family(&entry.finding),
                entry_index,
                state,
                detail,
                reason: entry.reason.clone(),
                members: vec![locator.clone()],
            }
        })
        .collect()
}

/// Numeric families share completed exact-visibility accounting with distinct keys.
pub(super) trait Visibility: summary::Decision {
    type Key: Ord;
    const CONTRACT: summary::Contract;
    fn hidden_key(&self) -> Option<Self::Key>;
}

#[derive(serde::Serialize)]
pub(super) struct Presentation<D: Visibility> {
    #[serde(flatten)]
    pub summary: summary::Summary,
    hidden_selected_findings: usize,
    decisions: Vec<D>,
    #[serde(skip)]
    pub hidden: std::collections::BTreeSet<D::Key>,
}

impl<D: Visibility> Presentation<D> {
    pub fn new(decisions: Vec<D>, retained: usize, scope: summary::Scope) -> Self {
        let hidden: std::collections::BTreeSet<_> = decisions
            .iter()
            .filter_map(Visibility::hidden_key)
            .collect();
        Self {
            summary: summary::summarize(&decisions, retained, scope, D::CONTRACT),
            hidden_selected_findings: hidden.len(),
            decisions,
            hidden,
        }
    }
}
