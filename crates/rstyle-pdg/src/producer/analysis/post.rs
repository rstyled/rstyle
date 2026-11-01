use super::*;

// The postdominator tree stores O(V log V) indices instead of O(R) strings.
pub(super) struct PostTree {
    #[cfg(test)]
    pub(super) ids: Vec<String>,
    parent: Vec<usize>,
    depth: Vec<usize>,
    pub(super) jumps: Vec<Vec<usize>>,
}

impl PostTree {
    pub(super) fn new(graph: &Topology, order: &[usize]) -> Self {
        let mut tree = Self::empty(graph, order[0]);
        tree.attach_order(graph, order);

        tree
    }

    pub(super) fn empty(graph: &Topology, root: usize) -> Self {
        let size = graph.ids.len();
        let levels = (usize::BITS - size.leading_zeros()) as usize;

        Self {
            #[cfg(test)]
            ids: graph.ids.clone(),
            parent: vec![root; size],
            depth: vec![0; size],
            jumps: vec![vec![root; levels]; size],
        }
    }

    pub(super) fn attach_order(&mut self, graph: &Topology, order: &[usize]) {
        for &vertex in &order[1..] {
            let (first, rest) = graph.successors[vertex]
                .split_first()
                .expect("validated successor");
            let parent = rest
                .iter()
                .fold(*first, |shared, &next| self.common_ancestor(shared, next));
            self.attach(vertex, parent);
        }
    }

    pub(super) fn attach(&mut self, vertex: usize, parent: usize) {
        self.parent[vertex] = parent;
        self.depth[vertex] = self.depth[parent] + 1;
        self.fill_jumps(vertex, parent);
    }

    pub(super) fn fill_jumps(&mut self, vertex: usize, parent: usize) {
        self.jumps[vertex][0] = parent;
        for level in 1..self.jumps[vertex].len() {
            self.jumps[vertex][level] = self.jumps[self.jumps[vertex][level - 1]][level - 1];
        }
    }

    pub(super) fn lift(&self, mut vertex: usize, mut distance: usize) -> usize {
        while distance != 0 {
            let level = distance.trailing_zeros() as usize;
            vertex = self.jumps[vertex][level];
            distance &= distance - 1;
        }

        vertex
    }

    pub(super) fn common_ancestor(&self, left: usize, right: usize) -> usize {
        let (mut left, mut right) = if self.depth[left] >= self.depth[right] {
            (self.lift(left, self.depth[left] - self.depth[right]), right)
        } else {
            (left, self.lift(right, self.depth[right] - self.depth[left]))
        };
        if left == right {
            return left;
        }
        for level in (0..self.jumps[left].len()).rev() {
            if self.jumps[left][level] != self.jumps[right][level] {
                left = self.jumps[left][level];
                right = self.jumps[right][level];
            }
        }

        self.parent[left]
    }

    pub(super) fn contains_index(&self, vertex: usize, candidate: usize) -> bool {
        self.depth[vertex] >= self.depth[candidate]
            && self.lift(vertex, self.depth[vertex] - self.depth[candidate]) == candidate
    }

    pub(super) fn immediate_index(&self, vertex: usize) -> Option<usize> {
        (self.depth[vertex] != 0).then_some(self.parent[vertex])
    }

    #[cfg(test)]
    pub(super) fn contains(&self, vertex: &str, candidate: &str) -> bool {
        self.contains_index(
            self.ids
                .binary_search_by(|id| id.as_str().cmp(vertex))
                .unwrap(),
            self.ids
                .binary_search_by(|id| id.as_str().cmp(candidate))
                .unwrap(),
        )
    }

    #[cfg(test)]
    pub(super) fn immediate(&self, vertex: &str) -> Option<String> {
        let vertex = self
            .ids
            .binary_search_by(|id| id.as_str().cmp(vertex))
            .unwrap();
        self.immediate_index(vertex)
            .map(|parent| self.ids[parent].clone())
    }
}

pub(super) struct Admission {
    remaining: Vec<usize>,
    admitted: Vec<bool>,
    pub(super) queue: Vec<usize>,
}

impl Admission {
    pub(super) fn new(size: usize) -> Self {
        Self {
            remaining: vec![0; size],
            admitted: vec![false; size],
            queue: Vec::with_capacity(size),
        }
    }

    pub(super) fn seed(&mut self, candidate: usize, degrees: &[usize]) {
        self.reset(degrees);
        self.admitted[candidate] = true;
        self.queue.push(candidate);
    }

    pub(super) fn reset(&mut self, degrees: &[usize]) {
        self.remaining.copy_from_slice(degrees);
        self.admitted.fill(false);
        self.queue.clear();
    }

    pub(super) fn propagate(&mut self, predecessors: &[Vec<usize>]) {
        let mut head = 0;
        while head < self.queue.len() {
            let vertex = self.queue[head];
            head += 1;
            self.process(&predecessors[vertex]);
        }
    }

    pub(super) fn process(&mut self, predecessors: &[usize]) {
        for &vertex in predecessors {
            if self.admitted[vertex] {
                continue;
            }
            self.remaining[vertex] -= 1;
            if self.remaining[vertex] == 0 {
                self.admitted[vertex] = true;
                self.queue.push(vertex);
            }
        }
    }
}
