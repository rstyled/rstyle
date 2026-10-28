//! A bounded inventory of exact numeric findings, without live analysis authority.
use super::super::{
    Versions,
    density_rules::{self, Snapshot},
    local, local_adapter,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const SCHEMA: &str = "rstyle.density-baseline/1";
const MAX_ENTRIES: usize = 4096;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::clones) struct Baseline {
    pub schema: String,
    pub policy: density_rules::Policy,
    pub versions: Versions,
    pub contexts: Vec<local::BuildContext>,
    pub exclude: Vec<String>,
    pub entries: Vec<Snapshot>,
}

impl Baseline {
    pub(in crate::clones) fn new(
        entries: Vec<Snapshot>,
        policy: density_rules::Policy,
        mut contexts: Vec<local::BuildContext>,
        exclude: Vec<String>,
    ) -> Result<Self, String> {
        contexts.sort();
        contexts.dedup();

        let baseline = Self {
            schema: SCHEMA.into(),
            policy,
            versions: local_adapter::versions("side_effect_density"),
            contexts,
            exclude,
            entries,
        };
        baseline.validate()?;
        Ok(baseline)
    }

    pub(in crate::clones) fn validate(&self) -> Result<(), String> {
        self.validate_header()?;
        let mut identities = BTreeSet::new();
        for snapshot in &self.entries {
            density_rules::validate_snapshot(snapshot)
                .map_err(|error| format!("invalid density baseline finding: {error:?}"))?;
            if snapshot.policy != self.policy || snapshot.versions != self.versions {
                return Err("density baseline finding disagrees with policy or versions".into());
            }
            if self.contexts.binary_search(&snapshot.context).is_err() {
                return Err("density baseline finding has no retained build context".into());
            }
            if !identities.insert((&snapshot.locator, &snapshot.context.compilation_target)) {
                return Err("ambiguous original identity in density baseline".into());
            }
        }
        Ok(())
    }

    fn validate_header(&self) -> Result<(), String> {
        if self.schema != SCHEMA {
            return Err("unsupported density baseline schema".into());
        }
        if self.entries.len() > MAX_ENTRIES {
            return Err("density baseline exceeds 4096 entries".into());
        }

        if self.exclude.len() > MAX_ENTRIES {
            return Err("density baseline exceeds 4096 source exclusions".into());
        }
        rstyle_core::exclusions::Matcher::compile(&self.exclude)?;

        if self.contexts.len() > MAX_ENTRIES
            || !self.contexts.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err("density baseline build contexts are oversized or noncanonical".into());
        }
        for context in &self.contexts {
            local::validate_context(context)
                .map_err(|error| format!("invalid density baseline build context: {error:?}"))?;
        }

        self.policy
            .validate()
            .map_err(|error| format!("invalid density baseline policy: {error:?}"))?;
        if self.versions != local_adapter::versions("side_effect_density") {
            return Err("unsupported density baseline versions".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
