//! Saved density choices retain exact numeric policy, never analysis authority.
use super::{Locator, Versions, exact_rules, local};
use rstyle_core::{config::SideEffectsConfig, facts::EffectKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(super) type Error = super::ReviewError;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Policy {
    pub event_metric: String,
    pub statement_metric: String,
    pub code_lines_metric: String,
    pub classification_metric: String,
    pub config: SideEffectsConfig,
}

impl Policy {
    pub fn from_config(config: &SideEffectsConfig) -> Self {
        Self {
            event_metric: "rstyle.static-effect-events/1".into(),
            statement_metric: "rstyle.original-source-statements/1".into(),
            code_lines_metric: "rstyle.source-code-lines/1".into(),
            classification_metric: "rstyle.longest-native-path-prefix/1".into(),
            config: config.clone(),
        }
    }

    pub(in crate::clones) fn validate(&self) -> Result<(), Error> {
        if [
            &self.event_metric,
            &self.statement_metric,
            &self.code_lines_metric,
            &self.classification_metric,
        ]
        .iter()
        .any(|metric| metric.trim().is_empty())
            || self.config.transitive_depth != 2
        {
            return Err(Error::InvalidExpression);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Measurement {
    pub statements: usize,
    pub code_lines: usize,
    /// All present kinds have positive counts; absent kinds have zero events.
    pub event_counts: BTreeMap<EffectKind, usize>,
    pub actual: usize,
}

impl Measurement {
    fn validate(&self, config: &SideEffectsConfig) -> Result<(), Error> {
        if self.statements < config.min_statements
            || self.code_lines < config.min_lines
            || self.event_counts.contains_key(&EffectKind::Unknown)
            || self.event_counts.values().any(|count| *count == 0)
        {
            return Err(Error::InvalidExpression);
        }

        let weighted = self
            .event_counts
            .iter()
            .try_fold(0usize, |total, (kind, count)| {
                count
                    .checked_mul(config.weights.of(*kind))
                    .and_then(|term| total.checked_add(term))
            });
        let actual = weighted
            .and_then(|value| value.checked_mul(1000))
            .map(|value| value / self.statements.max(1));
        if actual != Some(self.actual) || self.actual <= config.max_density_milli {
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
// The live adapter additionally requires current completion and visibility.
pub(crate) type LiveFinding<'a> = exact_rules::LiveFinding<'a, Snapshot>;
pub(crate) type ValidatedStore<'a> = exact_rules::ValidatedStore<'a, Snapshot>;
pub(crate) type Lookup<'a> = exact_rules::Lookup<'a, Snapshot>;
pub(crate) use exact_rules::StaleReason;
pub(super) use exact_rules::{record_dismissal, validate_entries};

pub(super) fn validate_snapshot(snapshot: &Snapshot) -> Result<(), Error> {
    exact_rules::validate_identity(snapshot, "side_effect_density")?;
    snapshot.policy.validate()?;
    snapshot.measurement.validate(&snapshot.policy.config)
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
