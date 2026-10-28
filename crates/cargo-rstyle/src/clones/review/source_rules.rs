//! Exact parameter-rule choices contain measured signature policy, never body proof.
use super::{CompilationTarget, Locator, Versions, local};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(super) type Error = super::ReviewError;
const MAX_ENTRIES: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Policy {
    pub max_params: usize,
    pub count_self: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Signature {
    pub parameters: usize,
    pub has_self: bool,
    pub actual: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Snapshot {
    pub versions: Versions,
    pub locator: Locator,
    pub context: local::BuildContext,
    pub policy: Policy,
    pub signature: Signature,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub finding: Snapshot,
    pub reason: String,
}

/// Only the live adapter may mint this after a completed visible parameter finding.
pub(crate) struct LiveFinding<'a> {
    snapshot: &'a Snapshot,
}

impl<'a> LiveFinding<'a> {
    pub(super) fn checked(snapshot: &'a Snapshot) -> Result<Self, Error> {
        validate_snapshot(snapshot)?;
        Ok(Self { snapshot })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StaleReason {
    Versions,
    Context,
    Policy,
    Signature,
}

#[derive(Debug)]
pub(crate) struct StaleEntry<'a> {
    pub entry: &'a Entry,
    pub reason: StaleReason,
}

#[derive(Debug)]
pub(crate) enum Lookup<'a> {
    HiddenExact(&'a Entry),
    VisibleNew,
    Stale(Vec<StaleEntry<'a>>),
}

type EntryIndex<'a> = BTreeMap<(&'a Locator, &'a CompilationTarget), Vec<&'a Entry>>;

pub(crate) struct ValidatedStore<'a> {
    entries: EntryIndex<'a>,
}

impl<'a> ValidatedStore<'a> {
    pub fn new(entries: &'a [Entry]) -> Result<Self, Error> {
        validate_entries(entries)?;
        let mut index: EntryIndex<'a> = BTreeMap::new();
        for entry in entries {
            let finding = &entry.finding;
            index
                .entry((&finding.locator, &finding.context.compilation_target))
                .or_default()
                .push(entry);
        }
        Ok(Self { entries: index })
    }

    pub fn lookup(&self, current: &LiveFinding<'_>) -> Lookup<'a> {
        let snapshot = current.snapshot;
        let key = (&snapshot.locator, &snapshot.context.compilation_target);
        let Some(entries) = self.entries.get(&key) else {
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

pub(super) fn validate_snapshot(snapshot: &Snapshot) -> Result<(), Error> {
    let versions = &snapshot.versions;
    if versions.detector != "fn_params"
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
    local::validate_locator(&snapshot.locator)?;
    local::validate_context(&snapshot.context)?;

    let signature = &snapshot.signature;
    let actual = signature.parameters.checked_add(usize::from(
        snapshot.policy.count_self && signature.has_self,
    ));
    if actual != Some(signature.actual) || signature.actual <= snapshot.policy.max_params {
        return Err(Error::InvalidSignature);
    }
    Ok(())
}

pub(super) fn validate_entries(entries: &[Entry]) -> Result<(), Error> {
    if entries.len() > MAX_ENTRIES {
        return Err(Error::ExpressionBudget);
    }
    let mut seen: BTreeMap<_, Vec<&Snapshot>> = BTreeMap::new();
    for entry in entries {
        if entry.reason.trim().is_empty() {
            return Err(Error::InvalidReason);
        }
        validate_snapshot(&entry.finding)?;
        let key = (
            &entry.finding.locator,
            &entry.finding.context.compilation_target,
        );
        let previous = seen.entry(key).or_default();
        if previous.contains(&&entry.finding) {
            return Err(Error::DuplicateReview);
        }
        previous.push(&entry.finding);
    }
    Ok(())
}

pub(super) fn record_dismissal(
    entries: &mut Vec<Entry>,
    current: &LiveFinding<'_>,
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

pub(super) fn stale_reason(previous: &Snapshot, current: &Snapshot) -> Option<StaleReason> {
    if previous.locator != current.locator
        || previous.context.compilation_target != current.context.compilation_target
    {
        return None;
    }

    if previous.versions != current.versions {
        return Some(StaleReason::Versions);
    }
    if previous.context != current.context {
        return Some(StaleReason::Context);
    }
    if previous.policy != current.policy {
        return Some(StaleReason::Policy);
    }
    (previous.signature != current.signature).then_some(StaleReason::Signature)
}

#[cfg(test)]
mod tests;
