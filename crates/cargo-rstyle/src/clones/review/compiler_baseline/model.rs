//! Persist compact completed diagnostic values, never analysis authority.
use super::super::{
    Versions, baseline_common, baseline_io, compiler_diagnostics::Snapshot, local, local_adapter,
};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::clones) struct Baseline {
    schema: String,
    pub policy: Rc<serde_json::Value>,
    versions: Versions,
    metrics: String,
    pub contexts: Vec<local::BuildContext>,
    pub entries: Vec<Snapshot>,
}

impl baseline_common::CompilerInventory<super::super::compiler_diagnostics::observed::Policy<'_>>
    for Baseline
{
    fn empty(
        policy: super::super::compiler_diagnostics::observed::Policy<'_>,
        contexts: Vec<local::BuildContext>,
    ) -> Self {
        Self {
            schema: "rstyle.compiler-baseline/1".into(),
            policy: Rc::clone(policy.config),
            versions: local_adapter::versions("compiler-diagnostics"),
            metrics: policy.metrics.into(),
            contexts,
            entries: Vec::new(),
        }
    }
}

impl Baseline {
    fn validate_header(&self) -> Result<(), String> {
        if self.schema != "rstyle.compiler-baseline/1"
            || self.versions != local_adapter::versions("compiler-diagnostics")
            || rstyle_core::advisory::metric_version(&self.metrics).is_none()
        {
            return Err("Unsupported compiler baseline schema or versions".into());
        }
        if !self.policy.is_object() {
            return Err("Compiler baseline policy must be a complete observed object".into());
        }
        if self.entries.len() > 4096 || self.contexts.len() > 4096 {
            return Err("Compiler baseline exceeds 4096 entries or contexts".into());
        }

        if !self.contexts.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err("Compiler baseline build contexts are noncanonical".into());
        }
        for context in &self.contexts {
            local::validate_context(context)
                .map_err(|error| format!("Invalid compiler baseline build context: {error:?}"))?;
        }
        Ok(())
    }
}

impl baseline_io::Artifact for Baseline {
    const LABEL: &'static str = "Compiler diagnostic baseline";
    fn validate(&self) -> Result<(), String> {
        super::super::baseline_common::Inventory::validate_inventory(self)
    }
}

impl baseline_common::Inventory for Baseline {
    fn validate_header(&self) -> Result<(), String> {
        Baseline::validate_header(self)
    }
    fn validate_findings(&self) -> Result<(), String> {
        baseline_common::FindingInventory {
            contexts: &self.contexts,
            label: "compiler",
        }
        .validate(
            &self.entries,
            |finding| {
                finding.policy == self.policy
                    && finding.versions == self.versions
                    && finding.metrics == self.metrics
            },
            |finding| &finding.rule_id,
        )
    }
    type Entry = Snapshot;
    const REPORT_SCHEMA: &'static str = "rstyle.compiler-baseline-report/1";
    const INCOMPLETE: &'static str = "Compiler inputs, diagnostic identities or capture are incomplete; a baseline cannot establish clean analysis";
    fn same_scope(&self, previous: &Self) -> bool {
        self.policy == previous.policy
            && self.contexts == previous.contexts
            && self.metrics == previous.metrics
    }
    fn replace_entries(&mut self, entries: Vec<Snapshot>) {
        self.entries = entries;
    }
}
