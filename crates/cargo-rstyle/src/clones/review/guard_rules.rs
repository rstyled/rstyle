//! Exact advisory region choices; persisted source never grants guard evidence.
use super::{Locator, ReviewError, Versions, exact_rules, local};
use crate::clones::analysis::guards::Branch;
use serde::{Deserialize, Serialize};

pub(super) const MAX_SOURCE: usize = 256 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Region {
    FunctionTail,
    Closure { ordinal: usize },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Snapshot {
    pub versions: Versions,
    pub locator: Locator,
    pub context: local::BuildContext,
    pub region: Region,
    pub guard_source: String,
    #[serde(deserialize_with = "serde::Deserialize::deserialize")]
    pub closure_source: Option<String>,
    pub fallback_branch: Branch,
    pub continuing_statements: usize,
}

pub(crate) type Entry = exact_rules::Entry<Snapshot>;
pub(super) use exact_rules::validate_entries;

impl exact_rules::ReviewSnapshot for Snapshot {
    fn identity(&self) -> (&Locator, &local::BuildContext) {
        (&self.locator, &self.context)
    }
    fn versions(&self) -> &Versions {
        &self.versions
    }
    fn validate(&self) -> Result<(), ReviewError> {
        exact_rules::validate_identity(self, "guard_clause")?;
        if self.guard_source.is_empty() || self.guard_source.len() > MAX_SOURCE {
            return Err(ReviewError::ExpressionBudget);
        }

        let valid_region = match (&self.region, &self.closure_source) {
            (Region::FunctionTail, None) => true,
            (Region::Closure { ordinal }, Some(source)) => {
                *ordinal < exact_rules::MAX_ENTRIES
                    && source.len() <= MAX_SOURCE
                    && source.contains(&self.guard_source)
            }
            _ => false,
        };

        if !valid_region || self.continuing_statements < 2 {
            return Err(ReviewError::InvalidExpression);
        }
        Ok(())
    }
    fn same_scope(&self, current: &Self) -> bool {
        self.region == current.region
    }
    fn changed(&self, current: &Self) -> Option<exact_rules::StaleReason> {
        (self.guard_source != current.guard_source
            || self.closure_source != current.closure_source
            || self.fallback_branch != current.fallback_branch
            || self.continuing_statements != current.continuing_statements)
            .then_some(exact_rules::StaleReason::Measurement)
    }
}

impl super::positive_workflow::Snapshot for Snapshot {
    fn locators(&self) -> Vec<Locator> {
        vec![self.locator.clone()]
    }
}

#[cfg(test)]
mod tests;
