//! Spacing choices retain numeric policy and counts across source edits.
//! A completed positive live detector establishes absence of a separating blank line;
//! numeric validation alone cannot establish that source property.
use super::{Locator, Versions, exact_rules, local};
use serde::{Deserialize, Serialize};

pub(super) type Error = super::ReviewError;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Policy {
    pub min_statements: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Measurement {
    pub direct_statements: usize,
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
/// The adapter must complete a positive spacing detection before constructing this view.
pub(crate) type LiveFinding<'a> = exact_rules::LiveFinding<'a, Snapshot>;
pub(crate) type ValidatedStore<'a> = exact_rules::ValidatedStore<'a, Snapshot>;
pub(crate) type Lookup<'a> = exact_rules::Lookup<'a, Snapshot>;
pub(crate) use exact_rules::StaleReason;
pub(super) use exact_rules::{record_dismissal, validate_entries};

pub(super) fn validate_snapshot(snapshot: &Snapshot) -> Result<(), Error> {
    exact_rules::validate_identity(snapshot, "fn_spacing")?;
    if snapshot.policy.min_statements == 0
        || snapshot.measurement.direct_statements < snapshot.policy.min_statements
    {
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
