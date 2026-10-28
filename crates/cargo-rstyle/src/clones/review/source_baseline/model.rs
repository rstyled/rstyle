//! Closed source-rule inventories retain comparison data without analysis authority.
use super::super::{
    Locator, Versions, length_rules, local, local_adapter, source_rules, spacing_rules,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const SCHEMA: &str = "rstyle.source-baseline/1";
const MAX_ENTRIES: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::clones) struct Policy {
    pub parameters: source_rules::Policy,
    pub lengths: length_rules::Policy,
    pub spacings: Option<spacing_rules::Policy>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::clones) struct Detectors {
    pub parameters: Versions,
    pub lengths: Versions,
    pub spacings: Versions,
}

impl Detectors {
    pub(in crate::clones) fn current() -> Self {
        Self {
            parameters: local_adapter::versions("fn_params"),
            lengths: local_adapter::versions("fn_length"),
            spacings: local_adapter::versions("fn_spacing"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "rule",
    content = "finding",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(in crate::clones) enum Finding {
    Parameter(source_rules::Snapshot),
    Length(length_rules::Snapshot),
    Spacing(spacing_rules::Snapshot),
}

impl Finding {
    pub(in crate::clones) fn identity(&self) -> (&Locator, &local::BuildContext) {
        match self {
            Self::Parameter(value) => (&value.locator, &value.context),
            Self::Length(value) => (&value.locator, &value.context),
            Self::Spacing(value) => (&value.locator, &value.context),
        }
    }

    pub(in crate::clones) fn rule_id(&self) -> &'static str {
        match self {
            Self::Parameter(_) => "fn_params",
            Self::Length(_) => "fn_length",
            Self::Spacing(_) => "fn_spacing",
        }
    }

    fn validate(&self) -> Result<(), String> {
        let result = match self {
            Self::Parameter(value) => source_rules::validate_snapshot(value),
            Self::Length(value) => length_rules::validate_snapshot(value),
            Self::Spacing(value) => spacing_rules::validate_snapshot(value),
        };
        result.map_err(|error| format!("invalid source baseline finding: {error:?}"))
    }

    fn matches_header(&self, policy: &Policy, versions: &Detectors) -> bool {
        match self {
            Self::Parameter(value) => {
                value.policy == policy.parameters && value.versions == versions.parameters
            }
            Self::Length(value) => {
                value.policy == policy.lengths && value.versions == versions.lengths
            }
            Self::Spacing(value) => {
                Some(&value.policy) == policy.spacings.as_ref()
                    && value.versions == versions.spacings
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::clones) struct Baseline {
    pub schema: String,
    pub policy: Policy,
    pub versions: Detectors,
    pub contexts: Vec<local::BuildContext>,
    pub exclude: Vec<String>,
    pub entries: Vec<Finding>,
}

impl Baseline {
    pub(in crate::clones) fn new(
        entries: Vec<Finding>,
        policy: Policy,
        contexts: Vec<local::BuildContext>,
        exclude: Vec<String>,
    ) -> Result<Self, String> {
        super::super::baseline_common::build(entries, contexts, |entries, contexts| Self {
            schema: SCHEMA.into(),
            policy,
            versions: Detectors::current(),
            contexts,
            exclude,
            entries,
        })
    }

    pub(in crate::clones) fn validate(&self) -> Result<(), String> {
        self.validate_header()?;
        let mut identities = BTreeSet::new();
        for finding in &self.entries {
            finding.validate()?;
            if !finding.matches_header(&self.policy, &self.versions) {
                return Err("source baseline finding disagrees with policy or versions".into());
            }
            let (locator, context) = finding.identity();
            if self.contexts.binary_search(context).is_err() {
                return Err("source baseline finding has no retained build context".into());
            }
            if !identities.insert((finding.rule_id(), locator, &context.compilation_target)) {
                return Err("ambiguous original identity in source baseline".into());
            }
        }
        Ok(())
    }

    fn validate_header(&self) -> Result<(), String> {
        if self.schema != SCHEMA || self.versions != Detectors::current() {
            return Err("unsupported source baseline schema or versions".into());
        }
        if self.entries.len() > MAX_ENTRIES || self.exclude.len() > MAX_ENTRIES {
            return Err("source baseline exceeds 4096 entries or exclusions".into());
        }
        rstyle_core::exclusions::Matcher::compile(&self.exclude)?;

        if self.contexts.len() > MAX_ENTRIES
            || !self.contexts.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err("source baseline build contexts are oversized or noncanonical".into());
        }
        for context in &self.contexts {
            local::validate_context(context)
                .map_err(|error| format!("invalid source baseline build context: {error:?}"))?;
        }
        if self
            .policy
            .spacings
            .as_ref()
            .is_some_and(|value| value.min_statements == 0)
        {
            return Err("source baseline spacing policy requires a positive minimum".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

impl super::super::baseline_common::Inventory for Baseline {
    const REPORT_SCHEMA: &'static str = "rstyle.source-baseline-report/1";
    const INCOMPLETE: &'static str = "Current source evidence, capture, identity or source policy is incomplete; a baseline cannot establish clean analysis";
    fn same_scope(&self, previous: &Self) -> bool {
        self.policy == previous.policy
            && self.contexts == previous.contexts
            && self.exclude == previous.exclude
    }
    type Entry = Finding;
    fn replace_entries(&mut self, entries: Vec<Self::Entry>) {
        self.entries = entries;
    }
}
