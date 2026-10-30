//! Saved numeric mutation choices retain counts, never current analysis authority.
use super::{Locator, Versions, exact_rules, local};
use serde::{Deserialize, Serialize};

pub(super) type Error = super::ReviewError;
const MAX_SITES: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Policy {
    MutationPerTarget { max_mutations_per_target: usize },
    MutatedTargets { max_mutated_targets: usize },
}

impl Policy {
    pub fn rule_id(&self) -> &'static str {
        match self {
            Self::MutationPerTarget { .. } => "mutation_per_target",
            Self::MutatedTargets { .. } => "mutated_targets",
        }
    }

    pub fn limit(&self) -> usize {
        match self {
            Self::MutationPerTarget {
                max_mutations_per_target,
            } => *max_mutations_per_target,
            Self::MutatedTargets {
                max_mutated_targets,
            } => *max_mutated_targets,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Measurement {
    /// Ascending nonzero counts preserve changes hidden by an unchanged maximum.
    pub sites_per_root: Vec<usize>,
}

impl Measurement {
    pub fn actual(&self, policy: &Policy) -> usize {
        match policy {
            Policy::MutationPerTarget { .. } => self.sites_per_root.last().copied().unwrap_or(0),
            Policy::MutatedTargets { .. } => self.sites_per_root.len(),
        }
    }

    fn validate(&self) -> Result<(), Error> {
        if self.sites_per_root.is_empty()
            || self.sites_per_root.len() > MAX_SITES
            || self.sites_per_root.contains(&0)
            || self.sites_per_root.windows(2).any(|pair| pair[0] > pair[1])
        {
            return Err(Error::InvalidExpression);
        }
        let sum = self
            .sites_per_root
            .iter()
            .try_fold(0usize, |total, count| total.checked_add(*count));
        if sum.is_none_or(|sum| sum > MAX_SITES) {
            return Err(Error::InvalidExpression);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Snapshot {
    pub versions: Versions,
    pub locator: Locator,
    pub context: local::BuildContext,
    pub policy: Policy,
    pub measurement: Measurement,
}

pub(crate) type Entry = exact_rules::Entry<Snapshot>;
// Loaded snapshots remain inert; the live adapter checks completion and visibility
// before minting this review-private handle from current analysis.
pub(crate) type LiveFinding<'a> = exact_rules::LiveFinding<'a, Snapshot>;
pub(crate) type ValidatedStore<'a> = exact_rules::ValidatedStore<'a, Snapshot>;
pub(crate) type Lookup<'a> = exact_rules::Lookup<'a, Snapshot>;
pub(crate) use exact_rules::StaleReason;
#[cfg(test)]
use exact_rules::record_dismissal;
pub(super) use exact_rules::validate_entries;

pub(super) fn validate_snapshot(snapshot: &Snapshot) -> Result<(), Error> {
    exact_rules::validate_identity(snapshot, snapshot.policy.rule_id())?;
    snapshot.measurement.validate()?;
    if snapshot.measurement.actual(&snapshot.policy) <= snapshot.policy.limit() {
        return Err(Error::InvalidExpression);
    }
    Ok(())
}

impl exact_rules::ReviewSnapshot for Snapshot {
    fn identity(&self) -> (&Locator, &local::BuildContext) {
        (&self.locator, &self.context)
    }

    fn versions(&self) -> &Versions {
        &self.versions
    }

    fn validate(&self) -> Result<(), Error> {
        validate_snapshot(self)
    }

    fn changed(&self, current: &Self) -> Option<StaleReason> {
        if self.policy != current.policy {
            return Some(StaleReason::Policy);
        }
        (self.measurement != current.measurement).then_some(StaleReason::Measurement)
    }
}

#[cfg(test)]
mod tests;
