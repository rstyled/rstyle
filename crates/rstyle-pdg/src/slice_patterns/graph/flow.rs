//! Reachability and cut checks operate on actual serialized CFG transfers.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Flow<'a> {
    pair: PairView<'a>,
    mapping: BTreeMap<String, String>,
    traversal: Traversal<'a>,
}

impl<'a> Flow<'a> {
    pub(super) fn new(pair: PairView<'a>) -> Result<Self, String> {
        let mut mapping = BTreeMap::new();
        for entry in crate::read_operation_map(&pair.pdg.extensions)? {
            if mapping
                .insert(entry.pdg_node_id, entry.cfg_node_id)
                .is_some()
            {
                return Err("Prefix graph requires unique operation mappings".into());
            }
        }
        let traversal = Traversal::new(
            &pair.cfg.entry_node_id,
            pair.cfg
                .edges
                .iter()
                .map(|edge| (edge.source.as_str(), edge.target.as_str(), edge.kind)),
        );
        Ok(Self {
            pair,
            mapping,
            traversal,
        })
    }

    pub(super) fn operation(&self, node: &PdgNode) -> Result<&str, String> {
        self.mapping
            .get(&node.id)
            .map(String::as_str)
            .ok_or("Prefix graph operation lacks CFG mapping".into())
    }

    pub(super) fn outgoing(
        &self,
        node: &PdgNode,
        expected: &[(CfgEdgeKind, &PdgNode)],
    ) -> Result<(), String> {
        self.transfers(node, expected, false)
    }

    pub(super) fn incoming_false(
        &self,
        node: &PdgNode,
        selectors: &[&PdgNode],
    ) -> Result<(), String> {
        let expected: Vec<_> = selectors
            .iter()
            .map(|node| (CfgEdgeKind::False, *node))
            .collect();
        self.transfers(node, &expected, true)
    }

    fn transfers(
        &self,
        node: &PdgNode,
        expected: &[(CfgEdgeKind, &PdgNode)],
        incoming: bool,
    ) -> Result<(), String> {
        let anchor = self.operation(node)?;
        let expected = expected
            .iter()
            .map(|(kind, node)| self.operation(node).map(|id| (*kind, id)))
            .collect::<Result<Vec<_>, _>>()?;
        let actual = self.pair.cfg.edges.iter().filter_map(|edge| {
            let (endpoint, opposite) = if incoming {
                (&edge.target, &edge.source)
            } else {
                (&edge.source, &edge.target)
            };
            (endpoint == anchor).then_some((edge.kind, opposite.as_str()))
        });

        exact_transfers(actual, &expected)
    }

    pub(super) fn guarded(
        &self,
        selector: &PdgNode,
        node: &PdgNode,
        outcome: CfgEdgeKind,
    ) -> Result<(), String> {
        let selector = self.operation(selector)?;
        let target = self.operation(node)?;
        if !self.traversal.reaches(target, None, None)
            || self
                .traversal
                .reaches(target, Some((selector, outcome)), None)
        {
            return Err(
                "Prefix operation is unreachable or bypasses its required selector edge".into(),
            );
        }
        Ok(())
    }

    pub(super) fn before(&self, earlier: &PdgNode, later: &PdgNode) -> Result<(), String> {
        let earlier = self.operation(earlier)?;
        let later = self.operation(later)?;
        if !self.traversal.reaches(earlier, None, None)
            || !self.traversal.reaches(later, None, None)
            || self.traversal.reaches(later, None, Some(earlier))
        {
            return Err("Prefix evaluation order is bypassable".into());
        }
        Ok(())
    }
}

/// Per-verification adjacency preserves every edge, including duplicate kinds.
/// Traversals inspect only outgoing edges instead of rescanning the whole CFG.
fn exact_transfers<'a>(
    actual: impl Iterator<Item = (CfgEdgeKind, &'a str)>,
    expected: &[(CfgEdgeKind, &str)],
) -> Result<(), String> {
    let actual: Vec<_> = actual.collect();
    if !crate::validate::unique_inventory(&actual, expected) {
        return Err("Prefix CFG changes its exact unique transfer inventory".into());
    }

    Ok(())
}

struct Traversal<'a> {
    entry: &'a str,
    outgoing: BTreeMap<&'a str, Vec<(&'a str, CfgEdgeKind)>>,
}

impl<'a> Traversal<'a> {
    fn new(
        entry: &'a str,
        edges: impl IntoIterator<Item = (&'a str, &'a str, CfgEdgeKind)>,
    ) -> Self {
        let mut outgoing: BTreeMap<&str, Vec<(&str, CfgEdgeKind)>> = BTreeMap::new();
        for (source, target, kind) in edges {
            outgoing.entry(source).or_default().push((target, kind));
        }
        Self { entry, outgoing }
    }

    fn reaches(
        &self,
        target: &str,
        removed: Option<(&str, CfgEdgeKind)>,
        stop: Option<&str>,
    ) -> bool {
        let mut pending = vec![self.entry];
        let mut seen = BTreeSet::new();
        while let Some(node) = pending.pop() {
            if Some(node) == stop || !seen.insert(node) {
                continue;
            }
            if node == target {
                return true;
            }
            if let Some(outgoing) = self.outgoing.get(node) {
                pending.extend(
                    outgoing
                        .iter()
                        .filter(|(_, kind)| removed != Some((node, *kind)))
                        .map(|(target, _)| *target),
                );
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use CfgEdgeKind::{False, Normal, True};

    #[test]
    fn ordered_chain_cut_blocks_descendants_but_preserves_unrelated_branch() {
        let graph = Traversal::new(
            "entry",
            [
                ("entry", "length", Normal),
                ("length", "byte", True),
                ("length", "none", False),
                ("byte", "digit", Normal),
                ("digit", "some", True),
                ("digit", "none", False),
            ],
        );
        assert!(graph.reaches("some", None, None));
        assert!(!graph.reaches("some", Some(("length", True)), None));
        assert!(graph.reaches("none", Some(("length", True)), None));
        assert!(!graph.reaches("some", None, Some("byte")));
        assert!(graph.reaches("none", None, Some("byte")));
        assert!(!graph.reaches("entry", None, Some("entry")));
    }

    #[test]
    fn bypass_and_cycles_cannot_create_a_false_guard_or_order_proof() {
        let graph = Traversal::new(
            "entry",
            [
                ("entry", "selector", Normal),
                ("selector", "read", True),
                ("selector", "bypass", False),
                ("bypass", "read", Normal),
                ("read", "selector", Normal),
                ("read", "exit", Normal),
                ("detached", "missing", Normal),
            ],
        );
        assert!(graph.reaches("read", Some(("selector", True)), None));
        assert!(graph.reaches("read", None, Some("bypass")));
        assert!(!graph.reaches("exit", None, Some("read")));
        assert!(!graph.reaches("detached", None, None));
        assert!(!graph.reaches("missing", None, None));
    }

    #[test]
    fn parallel_edges_keep_multiplicity_and_cut_all_selected_kind_edges() {
        let graph = Traversal::new(
            "entry",
            [
                ("entry", "selector", Normal),
                ("selector", "a", True),
                ("selector", "a", True),
                ("selector", "b", True),
                ("selector", "a", False),
                ("a", "unknown_target", Normal),
            ],
        );
        assert_eq!(graph.outgoing["selector"].len(), 4);
        assert!(graph.reaches("a", Some(("selector", True)), None));
        assert!(!graph.reaches("b", Some(("selector", True)), None));
        assert!(graph.reaches("unknown_target", None, None));
        assert!(!graph.reaches("unknown_target", None, Some("a")));
    }
}
