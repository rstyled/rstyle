//! Per-verification ordered lookups; never persisted or shared as authority.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Key = (u32, u32, u32);
pub(super) type Keys = BTreeSet<Key>;

pub(super) fn key(hir: &HirRef) -> Key {
    (hir.krate, hir.owner, hir.local)
}

pub(super) struct SourceIndex<'a> {
    pub nodes: Vec<&'a HirNode>,
    spans: BTreeMap<(u32, u32), Vec<&'a HirNode>>,
    identities: Keys,
}

impl<'a> SourceIndex<'a> {
    pub fn new(root: &'a HirNode) -> Result<Self, String> {
        let mut index = Self {
            nodes: Vec::new(),
            spans: BTreeMap::new(),
            identities: Keys::new(),
        };
        index.insert(root)?;
        Ok(index)
    }

    fn insert(&mut self, node: &'a HirNode) -> Result<(), String> {
        self.record(node)?;
        for child in &node.children {
            self.insert(child)?;
        }
        Ok(())
    }

    fn record(&mut self, node: &'a HirNode) -> Result<(), String> {
        let identity = key(&node.hir);
        let span = (node.span.start, node.span.end);

        if !self.identities.insert(identity) {
            return Err("Owned inventory has duplicate HIR identities".into());
        }

        self.nodes.push(node);
        self.spans.entry(span).or_default().push(node);
        Ok(())
    }

    pub fn unique(&self, span: ByteSpan, kind: &str) -> Result<&'a HirNode, String> {
        let mut found = self
            .spans
            .get(&(span.start, span.end))
            .into_iter()
            .flatten()
            .filter(|node| node.kind == kind);
        match (found.next(), found.next()) {
            (Some(node), None) => Ok(*node),
            _ => Err(format!(
                "Owned flow requires one exact {kind} source/HIR node"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node() -> HirNode {
        let facts: crate::HirFnFacts = serde_json::from_str(include_str!(
            "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
        ))
        .unwrap();
        let mut node = facts.body;
        node.children.clear();
        node
    }

    #[test]
    fn duplicate_hir_is_rejected_even_when_source_spans_differ() {
        let mut root = node();
        let mut duplicate = root.clone();
        duplicate.span.end += 1;
        root.children.push(duplicate);
        assert!(
            SourceIndex::new(&root)
                .err()
                .unwrap()
                .contains("duplicate HIR identities")
        );
    }

    #[test]
    fn overlapping_source_spans_preserve_kind_and_identity_ambiguity() {
        let mut root = node();
        let mut child = root.clone();
        child.hir.owner += 1;
        child.kind = "expr_path".into();
        root.children.push(child.clone());
        let index = SourceIndex::new(&root).unwrap();
        assert_eq!(index.unique(root.span, "expr_path").unwrap().hir, child.hir);
        child.hir.krate += 1;
        root.children.push(child);
        let index = SourceIndex::new(&root).unwrap();
        assert!(index.unique(root.span, "expr_path").is_err());
        assert!(index.unique(root.span, "expr_lit").is_err());
    }
}
