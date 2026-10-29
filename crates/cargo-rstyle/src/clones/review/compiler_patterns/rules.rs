//! Full observation equality is review identity; fingerprints never hide findings.
use super::super::{Locator, Versions, exact_rules, local};
use rstyle_core::{candidates::CandidateKind, cluster::Params, facts::Pdg};
use serde::{Deserialize, Serialize, de::Error};
use std::rc::Rc;

use super::super::compiler_owners::Signature;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Declaration {
    pub locator: Locator,
    pub signature: Signature,
    #[serde(deserialize_with = "serde::Deserialize::deserialize")]
    pub implements: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Body {
    #[serde(deserialize_with = "read_graph")]
    pub graph: Pdg,
    /// Approximate macro/closure nodes cannot describe hidden edits by themselves.
    #[serde(deserialize_with = "serde::Deserialize::deserialize")]
    pub opaque_source: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Member {
    pub declaration: Rc<Declaration>,
    pub body: Rc<Body>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Snapshot {
    pub(super) versions: Versions,
    pub(super) context: local::BuildContext,
    pub(in crate::clones::review) policy: Params,
    /// Older records were captured with no trait-hiding oracle.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(in crate::clones::review) hide_implements: Vec<String>,
    pub(in crate::clones::review) kind: CandidateKind,
    pub(super) sites: Vec<Member>,
    pub(super) definitions: Vec<Rc<Declaration>>,
}

pub(in crate::clones::review) type Entry = exact_rules::Entry<Snapshot>;
pub(in crate::clones::review) use exact_rules::validate_entries;

pub(in crate::clones::review) type Key<'a> = (
    &'static str,
    Vec<&'a Locator>,
    usize,
    &'a super::super::CompilationTarget,
);

impl Snapshot {
    pub(in crate::clones::review) fn baseline_key(&self) -> Key<'_> {
        (
            self.kind.name(),
            self.locators().collect(),
            self.sites.len(),
            &self.context.compilation_target,
        )
    }

    pub(in crate::clones::review) fn locators(&self) -> impl Iterator<Item = &Locator> {
        self.sites
            .iter()
            .map(|member| &member.declaration.locator)
            .chain(self.definitions.iter().map(|member| &member.locator))
    }
}

impl exact_rules::ReviewSnapshot for Snapshot {
    fn identity(&self) -> (&Locator, &local::BuildContext) {
        (&self.sites[0].declaration.locator, &self.context)
    }
    fn versions(&self) -> &Versions {
        &self.versions
    }
    fn validate(&self) -> Result<(), super::super::ReviewError> {
        use super::super::ReviewError;
        if self.sites.is_empty() || self.sites.len() + self.definitions.len() > 64 {
            return Err(ReviewError::NoncanonicalMembers);
        }

        exact_rules::validate_identity(self, "compiler-patterns")?;
        let sites: Vec<_> = self
            .sites
            .iter()
            .map(|site| &site.declaration.locator)
            .collect();
        let definitions: Vec<_> = self.definitions.iter().map(|item| &item.locator).collect();

        for locators in [&sites, &definitions] {
            if locators.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(ReviewError::NoncanonicalMembers);
            }
            for locator in locators.iter() {
                local::validate_locator(locator)?;
            }
        }
        Ok(())
    }
    fn changed(&self, current: &Self) -> Option<exact_rules::StaleReason> {
        if self.policy != current.policy || self.hide_implements != current.hide_implements {
            return Some(exact_rules::StaleReason::Policy);
        }
        (self.kind != current.kind
            || self.sites != current.sites
            || self.definitions != current.definitions)
            .then_some(exact_rules::StaleReason::Measurement)
    }
}

impl super::super::positive_workflow::Snapshot for Snapshot {
    fn locators(&self) -> Vec<super::super::Locator> {
        self.locators().cloned().collect()
    }
}

fn read_graph<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Pdg, D::Error> {
    let raw = serde_json::Value::deserialize(deserializer)?;
    let graph = rstyle_core::advisory::patterns::read_graph(&raw).map_err(D::Error::custom)?;
    if graph.nodes.iter().any(|node| node.line != 0) {
        return Err(D::Error::custom(
            "Compiler review graph contains source positions",
        ));
    }
    Ok(graph)
}

#[cfg(test)]
mod tests;
