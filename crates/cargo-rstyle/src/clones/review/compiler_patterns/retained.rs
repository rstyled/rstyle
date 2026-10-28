//! Reuse completed positive lookups; bounded discovery cannot authorize pruning.
use super::super::{
    ReviewStore,
    exact_rules::{self, ReviewSnapshot},
    status,
};
use super::{Entry, observed::Row, rules::Snapshot};

type Lookup<'a> = Result<exact_rules::Lookup<'a, Snapshot>, String>;

pub(super) fn records(
    store: &ReviewStore,
    rows: &[Row<'_>],
    lookups: &[Lookup<'_>],
    requested: bool,
) -> Vec<status::Record> {
    store
        .compiler_pattern_entries
        .iter()
        .enumerate()
        .map(|(entry_index, entry)| {
            let state = if requested {
                assess(entry, rows, lookups)
            } else {
                status::State::NotRequested
            };
            status::Record {
                family: status::Family::CompilerPattern,
                entry_index,
                state,
                detail: detail(state).into(),
                reason: entry.reason.clone(),
                members: entry.finding.locators().cloned().collect(),
            }
        })
        .collect()
}

fn assess(entry: &Entry, rows: &[Row<'_>], lookups: &[Lookup<'_>]) -> status::State {
    let mut state = status::State::Unavailable;
    for (row, lookup) in rows.iter().zip(lookups) {
        match lookup {
            Ok(exact_rules::Lookup::HiddenExact(found)) if std::ptr::eq(entry, *found) => {
                return status::State::HiddenExact;
            }
            Ok(exact_rules::Lookup::Stale(stale)) => {
                for found in stale
                    .iter()
                    .filter(|found| std::ptr::eq(entry, found.entry))
                {
                    if found.reason == exact_rules::StaleReason::Context {
                        if state != status::State::Stale {
                            state = status::State::ContextChanged;
                        }
                    } else {
                        state = status::State::Stale;
                    }
                }
            }
            _ => {}
        }
        if state == status::State::Unavailable
            && lookup.is_ok()
            && let Ok(snapshot) = row.snapshot
            && entry.finding.identity().0 == snapshot.identity().0
        {
            state = if entry.finding.context == snapshot.context {
                status::State::Stale
            } else {
                status::State::ContextChanged
            };
        }
    }
    state
}

fn detail(state: status::State) -> &'static str {
    match state {
        status::State::HiddenExact => "Exact compiler proposal snapshot",
        status::State::Stale => "Compiler proposal policy, membership or observations changed",
        status::State::ContextChanged => "Proposal observed in a different compilation context",
        status::State::NotRequested => "Compiler proposals were not requested",
        _ => {
            "No matching positive proposal; bounded heuristic discovery cannot establish a completed negative"
        }
    }
}
