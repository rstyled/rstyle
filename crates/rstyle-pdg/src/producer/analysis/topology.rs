use super::*;
use std::collections::HashMap;

/// Lexical indices preserve deterministic set traversal independently of input order.
pub(super) struct Topology {
    pub ids: Vec<String>,
    pub indices: HashMap<String, usize>,
    pub successors: Vec<Vec<usize>>,
    pub predecessors: Vec<Vec<usize>>,
}

pub(super) fn reachable(graph: &Graph) -> Vec<String> {
    Topology::from_graph(graph).ids
}

impl Topology {
    pub fn from_graph(graph: &Graph) -> Self {
        let ids = std::iter::once("E")
            .chain(
                graph
                    .transfers
                    .iter()
                    .flat_map(|transfer| [transfer.source.as_str(), transfer.target.as_str()]),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut result = Self::empty(ids);
        for transfer in &graph.transfers {
            let source = result.indices[&transfer.source];
            let target = result.indices[&transfer.target];
            result.successors[source].push(target);
        }

        result.finish_adjacency();
        result.restrict_reachable()
    }

    fn restrict_reachable(self) -> Self {
        let mut reached = vec![false; self.ids.len()];
        let root = self.indices["E"];
        reached[root] = true;
        let mut queue = vec![root];
        let mut head = 0;

        while head < queue.len() {
            let vertex = queue[head];
            head += 1;
            for &target in &self.successors[vertex] {
                if !reached[target] {
                    reached[target] = true;
                    queue.push(target);
                }
            }
        }
        if queue.len() == self.ids.len() {
            return self;
        }
        self.restrict(&reached)
    }

    fn restrict(self, reached: &[bool]) -> Self {
        let ids = self
            .ids
            .iter()
            .zip(reached)
            .filter(|(_, reached)| **reached)
            .map(|(id, _)| id.clone())
            .collect();
        let mut graph = Self::empty(ids);
        let indices: Vec<_> = self
            .ids
            .iter()
            .map(|id| graph.indices.get(id).copied())
            .collect();
        for (source, targets) in self.successors.iter().enumerate() {
            if let Some(source) = indices[source] {
                graph.successors[source] = targets
                    .iter()
                    .map(|&target| indices[target].expect("reachable successor"))
                    .collect();
            }
        }

        graph.finish_adjacency();
        graph
    }

    fn empty(ids: Vec<String>) -> Self {
        let size = ids.len();
        let indices = ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), i))
            .collect();
        Self {
            ids,
            indices,
            successors: vec![Vec::new(); size],
            predecessors: vec![Vec::new(); size],
        }
    }

    fn finish_adjacency(&mut self) {
        for (source, targets) in self.successors.iter_mut().enumerate() {
            targets.sort_unstable();
            targets.dedup();
            for &target in targets.iter() {
                self.predecessors[target].push(source);
            }
        }
    }

    pub fn admitted_exit(&self) -> Vec<usize> {
        let Some(&exit) = self.indices.get("X") else {
            return Vec::new();
        };
        let degrees: Vec<_> = self.successors.iter().map(Vec::len).collect();

        let mut admission = Admission::new(self.ids.len());
        admission.seed(exit, &degrees);
        admission.propagate(&self.predecessors);
        admission.queue
    }

    pub fn divergence_relation(&self) -> Vec<Vec<usize>> {
        let degrees: Vec<_> = self.successors.iter().map(Vec::len).collect();
        let mut scratch = Admission::new(degrees.len());
        let mut relation = vec![Vec::new(); degrees.len()];
        for candidate in 0..degrees.len() {
            scratch.seed(candidate, &degrees);
            scratch.propagate(&self.predecessors);
            for &vertex in &scratch.queue {
                relation[vertex].push(candidate);
            }
        }

        relation
    }
}

#[cfg(test)]
impl Topology {
    pub fn new(vertices: &BTreeSet<String>, successors: &Sets) -> Self {
        let mut graph = Self::empty(vertices.iter().cloned().collect());
        for (source, targets) in successors {
            graph.successors[graph.indices[source]] =
                targets.iter().map(|id| graph.indices[id]).collect();
        }
        graph.finish_adjacency();
        graph
    }

    pub fn materialize(&self, relation: &[Vec<usize>]) -> Sets {
        self.ids
            .iter()
            .zip(relation)
            .map(|(id, set)| {
                (
                    id.clone(),
                    set.iter().map(|&i| self.ids[i].clone()).collect(),
                )
            })
            .collect()
    }

    pub fn exit_order(&self) -> Vec<usize> {
        let order = self.admitted_exit();
        assert_eq!(order.len(), self.ids.len(), "validated acyclic normal flow");
        order
    }
}
