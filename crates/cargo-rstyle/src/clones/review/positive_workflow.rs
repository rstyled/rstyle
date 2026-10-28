//! Positive review lookups and retained status share completed equality outcomes.
use super::{
    Locator,
    exact_rules::{self, ReviewSnapshot},
    outcome, status,
};

pub(super) trait Snapshot: ReviewSnapshot {
    fn locators(&self) -> Vec<Locator>;
}

pub(super) trait Row {
    type Snapshot: Snapshot;
    fn snapshot(&self) -> &Result<Self::Snapshot, String>;
}

use std::collections::BTreeMap;

type Lookup<'a, S> = Result<exact_rules::Lookup<'a, S>, String>;

pub(super) struct Contract {
    pub requested: bool,
    /// Skip positive-only records when a numeric consumer supplies completed owner scopes.
    pub retain_records: bool,
    pub family: status::Family,
    pub label: &'static str,
}

pub(super) struct Evaluation {
    pub outcomes: Vec<outcome::Outcome>,
    pub records: Vec<status::Record>,
}

pub(super) fn evaluate<R: Row>(
    entries: &[exact_rules::Entry<R::Snapshot>],
    rows: &[R],
    contract: Contract,
) -> Result<Evaluation, String> {
    let checked =
        exact_rules::ValidatedStore::new(entries).map_err(|error| format!("{error:?}"))?;
    let lookups: Vec<_> = rows
        .iter()
        .map(|row| {
            let snapshot = row.snapshot().as_ref().map_err(Clone::clone)?;
            let live = exact_rules::LiveFinding::checked(snapshot)
                .map_err(|error| format!("{error:?}"))?;
            Ok(checked.lookup(&live))
        })
        .collect();
    let records = records(entries, rows, &lookups, &contract);
    Ok(Evaluation {
        outcomes: lookups
            .into_iter()
            .map(|lookup| outcome::Outcome::checked(lookup.map(Into::into)))
            .collect(),
        records,
    })
}

fn records<R: Row>(
    entries: &[exact_rules::Entry<R::Snapshot>],
    rows: &[R],
    lookups: &[Lookup<'_, R::Snapshot>],
    contract: &Contract,
) -> Vec<status::Record> {
    if !contract.retain_records || entries.is_empty() {
        return Vec::new();
    }
    let states = States::completed(rows, lookups);
    entries
        .iter()
        .enumerate()
        .map(|(entry_index, entry)| {
            let state = if contract.requested {
                states.assess(entry)
            } else {
                status::State::NotRequested
            };
            contract.record(entry, entry_index, state)
        })
        .collect()
}

impl Contract {
    fn record<S: Snapshot>(
        &self,
        entry: &exact_rules::Entry<S>,
        entry_index: usize,
        state: status::State,
    ) -> status::Record {
        status::Record {
            family: self.family,
            entry_index,
            state,
            detail: format!("{}: {}", self.label, detail(state)),
            reason: entry.reason.clone(),
            members: entry.finding.locators(),
        }
    }
}

/// Entry addresses are scoped to the immutably borrowed store; never persisted or dereferenced.
struct States<'a, S> {
    completed: BTreeMap<*const exact_rules::Entry<S>, status::State>,
    current: BTreeMap<&'a Locator, Vec<&'a S>>,
}

impl<'a, S: Snapshot> States<'a, S> {
    fn completed<R: Row<Snapshot = S>>(rows: &'a [R], lookups: &[Lookup<'_, S>]) -> Self {
        let mut states = Self {
            completed: BTreeMap::new(),
            current: BTreeMap::new(),
        };
        for (row, lookup) in rows.iter().zip(lookups) {
            if lookup.is_ok()
                && let Ok(snapshot) = row.snapshot()
            {
                states
                    .current
                    .entry(snapshot.identity().0)
                    .or_default()
                    .push(snapshot);
            }
            match lookup {
                Ok(exact_rules::Lookup::HiddenExact(entry)) => {
                    states.retain(entry, status::State::HiddenExact)
                }
                Ok(exact_rules::Lookup::Stale(entries)) => {
                    for found in entries {
                        let state = if found.reason == exact_rules::StaleReason::Context {
                            status::State::ContextChanged
                        } else {
                            status::State::Stale
                        };
                        states.retain(found.entry, state);
                    }
                }
                _ => {}
            }
        }
        states
    }

    fn retain(&mut self, entry: &exact_rules::Entry<S>, state: status::State) {
        self.completed
            .entry(std::ptr::from_ref(entry))
            .and_modify(|previous| *previous = strongest(*previous, state))
            .or_insert(state);
    }

    fn assess(&self, entry: &exact_rules::Entry<S>) -> status::State {
        let mut state = self
            .completed
            .get(&std::ptr::from_ref(entry))
            .copied()
            .unwrap_or(status::State::Unavailable);
        if state == status::State::HiddenExact {
            return state;
        }

        let (locator, context) = entry.finding.identity();
        for current in self
            .current
            .get(locator)
            .into_iter()
            .flatten()
            .filter(|current| entry.finding.same_scope(current))
        {
            let observed = if context == current.identity().1 {
                status::State::Stale
            } else {
                status::State::ContextChanged
            };
            state = strongest(state, observed);
        }
        state
    }
}

fn strongest(first: status::State, second: status::State) -> status::State {
    use status::State;
    match (first, second) {
        (State::HiddenExact, _) | (_, State::HiddenExact) => State::HiddenExact,
        (State::Stale, _) | (_, State::Stale) => State::Stale,
        (State::ContextChanged, _) | (_, State::ContextChanged) => State::ContextChanged,
        _ => State::Unavailable,
    }
}

fn detail(state: status::State) -> &'static str {
    match state {
        status::State::HiddenExact => "Exact finding snapshot",
        status::State::Stale => "Policy, membership or observations changed",
        status::State::ContextChanged => "Finding observed in a different compilation context",
        status::State::NotRequested => "Analysis was not requested",
        _ => "No matching positive finding; this adapter does not establish completed negatives",
    }
}
