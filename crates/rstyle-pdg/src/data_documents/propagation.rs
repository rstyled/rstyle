//! Shared document inventory plumbing; each propagation keeps its semantic proof.
use super::{conditional_payloads::Guards, *};
use crate::{HirNode, unit_store::EvidenceFacts};

type Invocation = fn(PairView<'_>, &HirNode, &dyn EvidenceFacts, &Guards<'_>) -> Result<(), String>;

pub(super) struct Contract<const N: usize> {
    pub key: &'static str,
    pub constructs: [&'static str; N],
    pub capability: &'static str,
    pub policy_error: &'static str,
    pub candidate: fn(&HirNode, &dyn EvidenceFacts) -> bool,
    pub semantics: fn(&str) -> serde_json::Value,
    pub invocation: Invocation,
}

pub(super) struct AnnotationInventory {
    pub witness: &'static str,
    pub error: &'static str,
}

impl<const N: usize> Contract<N> {
    fn sources<'a>(&self, facts: &'a dyn EvidenceFacts) -> Vec<&'a HirNode> {
        crate::field_locations::selected_operations(facts, &["expr_try_desugar"], self.candidate)
    }

    pub fn verify_document_policy(
        &self,
        profile: &crate::ProfileId,
        facts: &dyn EvidenceFacts,
    ) -> Result<(), String> {
        crate::extraction_profile::require_document(
            profile,
            self.capability,
            || !self.sources(facts).is_empty(),
            self.policy_error,
        )
    }

    pub fn install(&self, pair: &mut Pair) {
        for node in &mut pair.pdg.nodes {
            if let Some(value) = self.annotation(node) {
                node.annotations.insert(self.key.into(), value);
            }
        }
    }

    fn annotation(&self, node: &PdgNode) -> Option<serde_json::Value> {
        let construct = node.attributes.construct.as_deref()?;
        self.constructs
            .contains(&construct)
            .then(|| (self.semantics)(construct))
    }

    pub fn verify_annotations(
        &self,
        pair: PairView<'_>,
        facts: &dyn EvidenceFacts,
        inventory: AnnotationInventory,
    ) -> Result<(), String> {
        self.verify(pair, facts)?;

        let expected = self.sources(facts).len() * N;
        if [self.key, inventory.witness].into_iter().any(|key| {
            pair.pdg
                .nodes
                .iter()
                .filter(|node| node.annotations.contains_key(key))
                .count()
                != expected
        }) {
            return Err(inventory.error.into());
        }
        Ok(())
    }

    pub fn verify(&self, pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
        let sources = self.sources(facts);
        super::verify_construct_inventory(pair, self.constructs, [sources.len(); N])?;
        self.verify_document_policy(&pair.pdg.profile, facts)?;
        if sources.is_empty() {
            return Ok(());
        }

        let guards = Guards::new(pair)?;
        for source in sources {
            (self.invocation)(pair, source, facts, &guards)?;
        }
        Ok(())
    }
}

/// Select a dedicated contract's rule without mixing its interpretation table.
pub(super) fn semantic_rule<const N: usize>(
    construct: &str,
    constructs: &[&str; N],
    rules: &std::sync::OnceLock<[serde_json::Value; N]>,
    initialize: fn() -> [serde_json::Value; N],
) -> serde_json::Value {
    let Some(position) = constructs
        .iter()
        .position(|candidate| *candidate == construct)
    else {
        return serde_json::Value::Null;
    };

    rules.get_or_init(initialize)[position].clone()
}
