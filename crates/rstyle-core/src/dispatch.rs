//! Enum-style dispatch inside one function: a `match` whose arms call analogous methods
//! (same signature class, different callees) on the value bound by the arm pattern.
//! This is the closest thing to a trait that already exists in the source.

use crate::facts::{EdgeKind, NodeKind, Pdg, PdgEdge};
use crate::wl::call_class;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dispatch {
    pub line: usize,
    /// Signature class shared by the arm calls.
    pub class: String,
    /// `(arm index, callee id)`, one per arm, by arm.
    pub arms: Vec<(usize, String)>,
}

/// PDG edges grouped once by source node. Every match node reads only its own
/// group, so `find` scans the edge list once instead of once per match and call.
struct EdgeIndex<'a> {
    /// Control edges leaving each node, in edge order.
    ctrl: BTreeMap<usize, Vec<&'a PdgEdge>>,
    /// `(from, to)`: node `to` receives the value of `from` as its receiver operand.
    receivers: BTreeSet<(usize, usize)>,
}

impl<'a> EdgeIndex<'a> {
    fn new(pdg: &'a Pdg) -> Self {
        let mut ctrl: BTreeMap<usize, Vec<&PdgEdge>> = BTreeMap::new();
        for e in pdg.edges.iter().filter(|e| e.kind == EdgeKind::Ctrl) {
            ctrl.entry(e.from).or_default().push(e);
        }
        let receivers = pdg
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Data && e.arg_pos == 0)
            .map(|e| (e.from, e.to))
            .collect();
        EdgeIndex { ctrl, receivers }
    }
}

/// Calls controlled by arm `arm` of match node `m` that receive the matched value.
fn arm_calls(pdg: &Pdg, index: &EdgeIndex<'_>, m: usize) -> Vec<(usize, String, String)> {
    index
        .ctrl
        .get(&m)
        .into_iter()
        .flatten()
        .filter(|e| pdg.nodes[e.to].kind == NodeKind::Call && index.receivers.contains(&(m, e.to)))
        .filter_map(|e| {
            let node = &pdg.nodes[e.to];
            Some((e.arg_pos, call_class(node), node.callee_id.clone()?))
        })
        .collect()
}

fn dispatches_of(pdg: &Pdg, index: &EdgeIndex<'_>, m: usize) -> Vec<Dispatch> {
    let mut by_class: BTreeMap<String, BTreeMap<usize, String>> = BTreeMap::new();
    for (arm, class, callee) in arm_calls(pdg, index, m) {
        by_class
            .entry(class)
            .or_default()
            .entry(arm)
            .or_insert(callee);
    }
    by_class
        .into_iter()
        .filter(|(_, arms)| arms.values().collect::<BTreeSet<_>>().len() >= 2)
        .map(|(class, arms)| Dispatch {
            line: pdg.nodes[m].line,
            class,
            arms: arms.into_iter().collect(),
        })
        .collect()
}

pub fn find(pdg: &Pdg) -> Vec<Dispatch> {
    let index = EdgeIndex::new(pdg);
    (0..pdg.nodes.len())
        .filter(|m| pdg.nodes[*m].kind == NodeKind::Match)
        .flat_map(|m| dispatches_of(pdg, &index, m))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::PdgNode;
    use crate::pdgtest::*;

    fn matcher(second: &str) -> Pdg {
        Pdg {
            nodes: vec![
                node(NodeKind::Param),
                node(NodeKind::Match),
                getter_call("Dog", "bark"),
                getter_call("Cat", second),
                node(NodeKind::Return),
            ],
            edges: vec![
                data(0, 1, 0),
                ctrl(1, 2, 0),
                ctrl(1, 3, 1),
                data(1, 2, 0),
                data(1, 3, 0),
                data(1, 4, 0),
            ],
        }
    }

    #[test]
    fn arms_calling_analogous_methods_are_a_dispatch() {
        let found = find(&matcher("meow"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].class, "fn(&Self) -> String");
        assert_eq!(
            found[0].arms,
            [
                (0, "k::Dog::bark".to_string()),
                (1, "k::Cat::meow".to_string())
            ]
        );
    }

    #[test]
    fn same_callee_in_every_arm_is_not() {
        let mut same = matcher("bark");
        same.nodes[3].callee_id = Some("k::Dog::bark".into());
        assert!(find(&same).is_empty());
    }

    #[test]
    fn each_match_reads_only_its_own_edges() {
        // Match 5 controls the same calls as match 1, but call 3 receives its value only
        // as a later operand, so match 5 has a single receiving arm.
        let mut pdg = matcher("meow");
        pdg.nodes.push(PdgNode {
            line: 9,
            ..node(NodeKind::Match)
        });
        pdg.edges
            .extend([ctrl(5, 2, 0), ctrl(5, 3, 1), data(5, 2, 0), data(5, 3, 1)]);
        let found = find(&pdg);
        assert_eq!(found.iter().map(|d| d.line).collect::<Vec<_>>(), [1]);

        pdg.edges.push(data(5, 3, 0));
        let found = find(&pdg);
        assert_eq!(found.iter().map(|d| d.line).collect::<Vec<_>>(), [1, 9]);
        assert_eq!(found[0].arms, found[1].arms);
    }

    #[test]
    fn control_edges_from_other_nodes_are_not_arms() {
        let mut pdg = matcher("meow");
        // A non-match node controlling a call that also receives match 1's value.
        pdg.edges.push(ctrl(0, 2, 7));
        let found = find(&pdg);
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0]
                .arms
                .iter()
                .map(|(arm, _)| *arm)
                .collect::<Vec<_>>(),
            [0, 1]
        );
    }
}
