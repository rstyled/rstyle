//! Exact parameter-rule choices contain measured signature policy, never body proof.
use super::{Locator, Versions, exact_rules, local};
use serde::{Deserialize, Serialize};

pub(super) type Error = super::ReviewError;

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

pub(crate) type Entry = exact_rules::Entry<Snapshot>;
pub(crate) type LiveFinding<'a> = exact_rules::LiveFinding<'a, Snapshot>;
pub(crate) type ValidatedStore<'a> = exact_rules::ValidatedStore<'a, Snapshot>;
pub(crate) type Lookup<'a> = exact_rules::Lookup<'a, Snapshot>;
pub(crate) use exact_rules::StaleReason;
pub(super) use exact_rules::{record_dismissal, stale_reason, validate_entries};

pub(super) fn validate_snapshot(snapshot: &Snapshot) -> Result<(), Error> {
    exact_rules::validate_identity(snapshot, "fn_params")?;

    let signature = &snapshot.signature;
    let actual = signature.parameters.checked_add(usize::from(
        snapshot.policy.count_self && signature.has_self,
    ));
    if actual != Some(signature.actual) || signature.actual <= snapshot.policy.max_params {
        return Err(Error::InvalidSignature);
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
        (self.signature != current.signature).then_some(StaleReason::Signature)
    }
}

#[cfg(test)]
use exact_rules::MAX_ENTRIES;
#[cfg(test)]
mod tests;
