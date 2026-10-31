use super::{Topology, bits::Bits};
use std::collections::VecDeque;

pub(super) fn immediate(graph: &Topology) -> Vec<Option<usize>> {
    let root = graph.indices["E"];
    let size = graph.ids.len();
    let mut sets = vec![Bits::full(size); size];
    sets[root] = Bits::empty(size);
    sets[root].insert(root);
    let mut queue: VecDeque<_> = (0..size).collect();
    let mut queued = vec![true; size];
    while let Some(vertex) = queue.pop_front() {
        queued[vertex] = false;
        if vertex == root {
            continue;
        }
        let next = candidate(graph, &sets, vertex);
        if next != sets[vertex] {
            sets[vertex] = next;
            schedule(&graph.successors[vertex], &mut queued, &mut queue);
        }
    }

    let depths: Vec<_> = sets.iter().map(Bits::count).collect();
    sets.iter()
        .enumerate()
        .map(|(vertex, set)| {
            set.members()
                .filter(|&candidate| candidate != vertex)
                .max_by_key(|&candidate| depths[candidate])
        })
        .collect()
}

pub(super) fn schedule(vertices: &[usize], queued: &mut [bool], queue: &mut VecDeque<usize>) {
    for &vertex in vertices {
        if !queued[vertex] {
            queued[vertex] = true;
            queue.push_back(vertex);
        }
    }
}

fn candidate(graph: &Topology, sets: &[Bits], vertex: usize) -> Bits {
    let mut next = Bits::full(graph.ids.len());
    for &source in &graph.predecessors[vertex] {
        next.intersect(&sets[source]);
    }
    next.insert(vertex);
    next
}
