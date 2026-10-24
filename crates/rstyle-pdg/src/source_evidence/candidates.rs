//! Per-inventory search accelerator; span correspondence remains the authority.
use super::{ByteSpan, HirNode, HirRef, corresponding_span};
use std::collections::HashMap;

pub(crate) struct Candidates<'a> {
    kinds: HashMap<&'a str, Vec<Candidate<'a>>>,
}

#[derive(Default)]
struct Matches<'a> {
    first: Option<&'a HirNode>,
    ambiguous: bool,
}

struct Candidate<'a> {
    node: &'a HirNode,
    max_end: u32,
}

impl<'a> Candidates<'a> {
    pub(crate) fn new(body: &'a HirNode) -> Self {
        let mut kinds = HashMap::new();
        collect(body, &mut kinds);
        for nodes in kinds.values_mut() {
            nodes.sort_by_key(|candidate| candidate.node.span.start);
            augment(nodes);
        }
        Self { kinds }
    }

    pub(super) fn unique(&self, source: &str, kind: &str, span: ByteSpan) -> Option<HirRef> {
        self.search(source, kind, span).0
    }

    fn search(&self, source: &str, kind: &str, span: ByteSpan) -> (Option<HirRef>, usize) {
        let (node, visited) = self.search_node(source, kind, span);
        (node.map(|node| node.hir.clone()), visited)
    }

    pub(crate) fn unique_node(
        &self,
        source: &str,
        kind: &str,
        span: ByteSpan,
    ) -> Option<&'a HirNode> {
        self.search_node(source, kind, span).0
    }

    fn search_node(
        &self,
        source: &str,
        kind: &str,
        span: ByteSpan,
    ) -> (Option<&'a HirNode>, usize) {
        let mut query = Query {
            source,
            span,
            matches: Matches::default(),
            visited: 0,
        };
        if let Some(nodes) = self.kinds.get(kind) {
            query.visit(nodes);
        }
        let result = if query.matches.ambiguous {
            None
        } else {
            query.matches.first
        };
        (result, query.visited)
    }
}

fn collect<'a>(node: &'a HirNode, kinds: &mut HashMap<&'a str, Vec<Candidate<'a>>>) {
    let aliases: &[&str] = match node.kind.as_str() {
        "expr_struct" => &["expr_call", "expr_path"],
        "pat_binding" | "pat_constant" => &["pat_name"],
        _ => &[],
    };
    for kind in std::iter::once(node.kind.as_str()).chain(aliases.iter().copied()) {
        kinds.entry(kind).or_default().push(Candidate {
            node,
            max_end: node.span.end,
        });
    }
    for child in &node.children {
        collect(child, kinds);
    }
}

fn augment(nodes: &mut [Candidate<'_>]) -> u32 {
    let count = nodes.len();
    if count == 0 {
        return 0;
    }
    let middle = count / 2;
    let (left, rest) = nodes.split_at_mut(middle);
    let max_end = subtree_max(left, rest);

    nodes[middle].max_end = max_end;
    max_end
}

fn subtree_max(left: &mut [Candidate<'_>], rest: &mut [Candidate<'_>]) -> u32 {
    if left.is_empty() && rest.len() == 1 {
        return rest[0].node.span.end;
    }
    let (root, right) = rest.split_first_mut().expect("nonempty candidate range");
    let left_max = augment(left);
    let right_max = augment(right);

    root.node.span.end.max(left_max).max(right_max)
}

struct Query<'source, 'hir> {
    source: &'source str,
    span: ByteSpan,
    matches: Matches<'hir>,
    visited: usize,
}

impl<'hir> Query<'_, 'hir> {
    fn visit(&mut self, nodes: &[Candidate<'hir>]) {
        if nodes.is_empty() || self.matches.ambiguous {
            return;
        }
        self.visited += 1;
        self.visit_overlapping(nodes);
    }

    fn visit_overlapping(&mut self, nodes: &[Candidate<'hir>]) {
        let middle = nodes.len() / 2;
        let root = &nodes[middle];
        if root.max_end < self.span.end || nodes[0].node.span.start > self.span.start {
            return;
        }

        self.visit(&nodes[..middle]);
        self.record(root);
        self.visit(&nodes[middle + 1..]);
    }

    fn record(&mut self, root: &Candidate<'hir>) {
        if root.node.span.start <= self.span.start
            && corresponding_span(self.source, self.span, root.node.span)
        {
            self.matches.ambiguous = self.matches.first.is_some();
            self.matches.first = Some(root.node);
        }
    }
}

#[cfg(test)]
mod tests;
