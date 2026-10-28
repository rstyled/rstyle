//! Bounded exact review choices shared by independently validated numeric rules.
use super::{CompilationTarget, Locator, Versions, local};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(super) type Error = super::ReviewError;
pub(super) const MAX_ENTRIES: usize = 4096;

pub(crate) trait ReviewSnapshot: Clone + Eq {
    fn identity(&self) -> (&Locator, &local::BuildContext);
    fn versions(&self) -> &Versions;
    fn validate(&self) -> Result<(), Error>;
    fn changed(&self, current: &Self) -> Option<StaleReason>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry<S> {
    pub finding: S,
    pub reason: String,
}

/// Only live adapters may mint this after a completed visible numeric finding.
pub(crate) struct LiveFinding<'a, S> {
    snapshot: &'a S,
}

impl<'a, S: ReviewSnapshot> LiveFinding<'a, S> {
    pub(super) fn checked(snapshot: &'a S) -> Result<Self, Error> {
        snapshot.validate()?;
        Ok(Self { snapshot })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StaleReason {
    Versions,
    Context,
    Policy,
    Signature,
    Measurement,
}

#[derive(Debug)]
pub(crate) struct StaleEntry<'a, S> {
    pub entry: &'a Entry<S>,
    pub reason: StaleReason,
}

#[derive(Debug)]
pub(crate) enum Lookup<'a, S> {
    HiddenExact(&'a Entry<S>),
    VisibleNew,
    Stale(Vec<StaleEntry<'a, S>>),
}

type EntryIndex<'a, S> = BTreeMap<(&'a Locator, &'a CompilationTarget), Vec<&'a Entry<S>>>;

pub(crate) struct ValidatedStore<'a, S> {
    entries: EntryIndex<'a, S>,
}

impl<'a, S: ReviewSnapshot> ValidatedStore<'a, S> {
    pub fn new(entries: &'a [Entry<S>]) -> Result<Self, Error> {
        Self::new_filtered(entries, |_| true)
    }

    pub fn new_filtered(
        entries: &'a [Entry<S>],
        select: impl Fn(&Entry<S>) -> bool,
    ) -> Result<Self, Error> {
        validate_entries(entries)?;
        let mut index: EntryIndex<'a, S> = BTreeMap::new();
        for entry in entries.iter().filter(|entry| select(entry)) {
            let (locator, context) = entry.finding.identity();
            index
                .entry((locator, &context.compilation_target))
                .or_default()
                .push(entry);
        }
        Ok(Self { entries: index })
    }

    pub fn lookup(&self, current: &LiveFinding<'_, S>) -> Lookup<'a, S> {
        let snapshot = current.snapshot;
        let (locator, context) = snapshot.identity();
        let Some(entries) = self.entries.get(&(locator, &context.compilation_target)) else {
            return Lookup::VisibleNew;
        };
        if let Some(entry) = entries
            .iter()
            .copied()
            .find(|entry| entry.finding == *snapshot)
        {
            return Lookup::HiddenExact(entry);
        }

        let stale: Vec<_> = entries
            .iter()
            .copied()
            .filter_map(|entry| {
                stale_reason(&entry.finding, snapshot).map(|reason| StaleEntry { entry, reason })
            })
            .collect();
        if stale.is_empty() {
            Lookup::VisibleNew
        } else {
            Lookup::Stale(stale)
        }
    }
}

pub(super) fn validate_identity(
    snapshot: &impl ReviewSnapshot,
    detector: &str,
) -> Result<(), Error> {
    let versions = snapshot.versions();
    if versions.detector != detector
        || [
            versions.detector_version,
            versions.identity_version,
            versions.locator_version,
            versions.context_version,
        ]
        .contains(&0)
    {
        return Err(Error::InvalidVersion);
    }

    let (locator, context) = snapshot.identity();
    local::validate_locator(locator)?;
    local::validate_context(context)
}

pub(super) fn validate_entries<S: ReviewSnapshot>(entries: &[Entry<S>]) -> Result<(), Error> {
    if entries.len() > MAX_ENTRIES {
        return Err(Error::ExpressionBudget);
    }
    let mut seen: BTreeMap<_, Vec<&S>> = BTreeMap::new();
    for entry in entries {
        if entry.reason.trim().is_empty() {
            return Err(Error::InvalidReason);
        }
        entry.finding.validate()?;
        let (locator, context) = entry.finding.identity();
        let previous = seen
            .entry((locator, &context.compilation_target))
            .or_default();
        if previous.contains(&&entry.finding) {
            return Err(Error::DuplicateReview);
        }
        previous.push(&entry.finding);
    }
    Ok(())
}

pub(super) fn record_dismissal<S: ReviewSnapshot>(
    entries: &mut Vec<Entry<S>>,
    current: &LiveFinding<'_, S>,
    reason: String,
) -> Result<(), Error> {
    validate_entries(entries)?;
    if reason.trim().is_empty() {
        return Err(Error::InvalidReason);
    }
    if entries
        .iter()
        .any(|entry| entry.finding == *current.snapshot)
    {
        return Err(Error::DuplicateReview);
    }
    if entries.len() >= MAX_ENTRIES {
        return Err(Error::ExpressionBudget);
    }

    entries.push(Entry {
        finding: current.snapshot.clone(),
        reason,
    });
    Ok(())
}

pub(super) fn stale_reason<S: ReviewSnapshot>(previous: &S, current: &S) -> Option<StaleReason> {
    let (old_locator, old_context) = previous.identity();
    let (new_locator, new_context) = current.identity();
    if old_locator != new_locator
        || old_context.compilation_target != new_context.compilation_target
    {
        return None;
    }

    if previous.versions() != current.versions() {
        return Some(StaleReason::Versions);
    }
    if old_context != new_context {
        return Some(StaleReason::Context);
    }
    previous.changed(current)
}
