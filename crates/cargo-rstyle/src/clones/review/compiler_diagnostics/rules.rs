//! Persist completed numeric diagnostics, without body or graph fingerprints.
use super::super::{
    Locator, Versions, compiler_owners::Signature, exact_rules, local, local_adapter,
    positive_workflow,
};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::clones::review) struct Owner {
    pub locator: Locator,
    pub context: local::BuildContext,
    pub signature: Signature,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Snapshot {
    pub(in crate::clones::review) owner: Rc<Owner>,
    pub versions: Versions,
    pub metrics: String,
    pub policy: Rc<serde_json::Value>,
    pub rule_id: String,
    pub actual: usize,
    pub limit: usize,
    pub details: Vec<(String, usize)>,
}

pub(in crate::clones::review) type Entry = exact_rules::Entry<Snapshot>;
pub(in crate::clones::review) use exact_rules::validate_entries;

impl exact_rules::ReviewSnapshot for Snapshot {
    fn identity(&self) -> (&Locator, &local::BuildContext) {
        (&self.owner.locator, &self.owner.context)
    }
    fn versions(&self) -> &Versions {
        &self.versions
    }
    fn same_scope(&self, current: &Self) -> bool {
        self.rule_id == current.rule_id
    }
    fn validate(&self) -> Result<(), super::super::ReviewError> {
        use super::super::ReviewError;
        exact_rules::validate_identity(self, "compiler-diagnostics")?;
        if self.versions != local_adapter::versions("compiler-diagnostics")
            || rstyle_core::advisory::metric_version(&self.metrics).is_none()
        {
            return Err(ReviewError::InvalidVersion);
        }

        if !matches!(
            self.rule_id.as_str(),
            "fn_length"
                | "fn_params"
                | "fn_spacing"
                | "cyclomatic_complexity"
                | "cognitive_complexity"
                | "mutation_per_target"
                | "mutated_targets"
                | "side_effect_density"
        ) || self.actual <= self.limit
            || !self.policy.is_object()
        {
            return Err(ReviewError::InvalidSchema);
        }
        if is_complexity(&self.rule_id) && self.metrics != rstyle_core::advisory::METRICS {
            return Err(ReviewError::InvalidVersion);
        }
        Ok(())
    }
    fn changed(&self, current: &Self) -> Option<exact_rules::StaleReason> {
        if self.metrics != current.metrics || self.policy != current.policy {
            return Some(exact_rules::StaleReason::Policy);
        }
        if self.owner.signature != current.owner.signature {
            return Some(exact_rules::StaleReason::Signature);
        }
        (self.actual != current.actual
            || self.limit != current.limit
            || self.details != current.details)
            .then_some(exact_rules::StaleReason::Measurement)
    }
}

impl positive_workflow::Snapshot for Snapshot {
    fn locators(&self) -> Vec<Locator> {
        vec![self.owner.locator.clone()]
    }
}

pub(super) fn is_complexity(rule: &str) -> bool {
    matches!(rule, "cyclomatic_complexity" | "cognitive_complexity")
}

#[cfg(test)]
mod tests;
