//! Proposal inventories retain completed shared snapshots and explicit whole-scope policy.
use super::super::{
    Versions, baseline_common, baseline_io, compiler_patterns::observed::Snapshot,
    exact_rules::ReviewSnapshot, local, local_adapter,
};
use rstyle_core::cluster::Params;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::clones) struct Baseline {
    schema: String,
    pub compiler_policy: Rc<serde_json::Value>,
    pub proposal_policy: Params,
    versions: Versions,
    graph_schema: String,
    pub contexts: Vec<local::BuildContext>,
    pub entries: Vec<Snapshot>,
}

impl baseline_common::CompilerInventory for Baseline {
    fn empty(compiler_policy: Rc<serde_json::Value>, contexts: Vec<local::BuildContext>) -> Self {
        Self {
            schema: "rstyle.compiler-pattern-baseline/1".into(),
            compiler_policy,
            proposal_policy: Params::default(),
            versions: local_adapter::versions("compiler-patterns"),
            graph_schema: rstyle_core::advisory::patterns::KEY.into(),
            contexts,
            entries: Vec::new(),
        }
    }
}

impl Baseline {
    fn validate_header(&self) -> Result<(), String> {
        if self.schema != "rstyle.compiler-pattern-baseline/1"
            || self.versions != local_adapter::versions("compiler-patterns")
            || self.graph_schema != rstyle_core::advisory::patterns::KEY
            || self.proposal_policy != Params::default()
        {
            return Err("Unsupported compiler proposal baseline schema, versions or policy".into());
        }
        if !self.compiler_policy.is_object() {
            return Err("Compiler proposal baseline needs the full compiler policy".into());
        }
        if self.entries.len() > 4096 || self.contexts.len() > 4096 {
            return Err("Compiler proposal baseline exceeds 4096 entries or contexts".into());
        }

        if !self.contexts.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err("Compiler proposal baseline build contexts are noncanonical".into());
        }
        for context in &self.contexts {
            local::validate_context(context).map_err(|error| {
                format!("Invalid compiler proposal baseline context: {error:?}")
            })?;
        }
        Ok(())
    }
}

impl baseline_io::Artifact for Baseline {
    const LABEL: &'static str = "Compiler proposal baseline";
    fn validate(&self) -> Result<(), String> {
        baseline_common::Inventory::validate_inventory(self)
    }
}

impl baseline_common::Inventory for Baseline {
    type Entry = Snapshot;
    const REPORT_SCHEMA: &'static str = "rstyle.compiler-pattern-baseline-report/1";
    const INCOMPLETE: &'static str = "Compiler proposal inputs, identities or capture are incomplete; a baseline cannot establish clean analysis";
    fn validate_header(&self) -> Result<(), String> {
        Baseline::validate_header(self)
    }
    fn validate_findings(&self) -> Result<(), String> {
        baseline_common::FindingInventory {
            contexts: &self.contexts,
            label: "compiler proposal",
        }
        .validate_keys(
            &self.entries,
            |finding| {
                finding.versions() == &self.versions && finding.policy == self.proposal_policy
            },
            Snapshot::baseline_key,
        )
    }
    fn same_scope(&self, previous: &Self) -> bool {
        self.compiler_policy == previous.compiler_policy
            && self.proposal_policy == previous.proposal_policy
            && self.contexts == previous.contexts
    }
    fn replace_entries(&mut self, entries: Vec<Snapshot>) {
        self.entries = entries;
    }
}
