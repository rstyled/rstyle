//! Enum-style dispatch inside one function: a `match` whose arms call analogous methods
//! (same signature class, different callees) on the value bound by the arm pattern.
//! This is the closest thing to a trait that already exists in the source.

use crate::facts::{EdgeKind, NodeKind, Pdg};
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

/// Calls controlled by arm `arm` of match node `m` that receive the matched value.
fn arm_calls(pdg: &Pdg, m: usize) -> Vec<(usize, String, String)> {
    let receives = |call: usize| {
        pdg.edges
            .iter()
            .any(|e| e.from == m && e.to == call && e.kind == EdgeKind::Data && e.arg_pos == 0)
    };
    pdg.edges
        .iter()
        .filter(|e| e.from == m && e.kind == EdgeKind::Ctrl)
        .filter(|e| pdg.nodes[e.to].kind == NodeKind::Call && receives(e.to))
        .filter_map(|e| {
            let node = &pdg.nodes[e.to];
            Some((e.arg_pos, call_class(node), node.callee_id.clone()?))
        })
        .collect()
}

fn dispatches_of(pdg: &Pdg, m: usize) -> Vec<Dispatch> {
    let mut by_class: BTreeMap<String, BTreeMap<usize, String>> = BTreeMap::new();
    for (arm, class, callee) in arm_calls(pdg, m) {
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
    (0..pdg.nodes.len())
        .filter(|m| pdg.nodes[*m].kind == NodeKind::Match)
        .flat_map(|m| dispatches_of(pdg, m))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
