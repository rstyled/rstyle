use crate::{BindingFact, ByteSpan, HirFnFacts, HirNode, HirRef};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
mod tests;

type Nodes<'a> = BTreeMap<(&'a str, u32, u32), Vec<&'a HirNode>>;

pub(super) struct Index<'a> {
    nodes: Nodes<'a>,
    bindings: BTreeMap<(u32, u32, u32), &'a BindingFact>,
}

impl<'a> Index<'a> {
    pub fn new(facts: &'a HirFnFacts) -> Result<Option<Self>, String> {
        if facts.bindings.len() > 16384 {
            return Ok(None);
        }
        let bindings = binding_index(facts)?;
        let Some(nodes) = Inventory::new(facts).collect(&facts.source.bytes)? else {
            return Ok(None);
        };

        Ok(Some(Self { nodes, bindings }))
    }

    pub fn node(&self, kind: &str, span: ByteSpan) -> Result<Option<&'a HirNode>, String> {
        Ok(self
            .captured_node(kind, span)?
            .filter(|node| node.expansion.is_none()))
    }

    pub fn captured_node(&self, kind: &str, span: ByteSpan) -> Result<Option<&'a HirNode>, String> {
        match self
            .nodes
            .get(&(kind, span.start, span.end))
            .map(Vec::as_slice)
        {
            None => Ok(None),
            Some([node]) => Ok(Some(*node)),
            Some(_) => Err("Mutation source site has ambiguous native correspondence".into()),
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.values().map(Vec::len).sum()
    }

    pub fn binding(&self, node: &HirNode) -> Result<&'a BindingFact, String> {
        self.bindings
            .get(&key(&node.hir))
            .copied()
            .ok_or_else(|| "Native declaration is absent from binding inventory".into())
    }
}

fn binding_index(facts: &HirFnFacts) -> Result<BTreeMap<(u32, u32, u32), &BindingFact>, String> {
    let mut bindings = BTreeMap::new();
    for binding in &facts.bindings {
        if bindings.insert(key(&binding.hir), binding).is_some() {
            return Err("Mutation binding inventory repeats a native identity".into());
        }
    }
    Ok(bindings)
}

struct Visit<'a> {
    node: &'a HirNode,
    depth: usize,
    guard_position: bool,
}

impl<'a> Visit<'a> {
    fn root(node: &'a HirNode) -> Self {
        Self {
            node,
            depth: 0,
            guard_position: false,
        }
    }
}

struct Inventory<'a> {
    nodes: Nodes<'a>,
    identities: BTreeSet<(u32, u32, u32)>,
    pending: Vec<Visit<'a>>,
    visited: usize,
}

impl<'a> Inventory<'a> {
    fn new(facts: &'a HirFnFacts) -> Self {
        let pending = facts
            .signature
            .params
            .iter()
            .map(|parameter| Visit::root(&parameter.pat))
            .chain(std::iter::once(Visit::root(&facts.body)))
            .collect();
        Self {
            nodes: BTreeMap::new(),
            identities: BTreeSet::new(),
            pending,
            visited: 0,
        }
    }

    fn collect(mut self, source: &str) -> Result<Option<Nodes<'a>>, String> {
        while let Some(visit) = self.pending.pop() {
            if !self.visit(visit, source)? {
                return Ok(None);
            }
        }
        Ok(Some(self.nodes))
    }

    fn visit(&mut self, visit: Visit<'a>, source: &str) -> Result<bool, String> {
        let Visit {
            node,
            depth,
            guard_position,
        } = visit;
        if self.visited >= 16384 || depth > 128 {
            return Ok(false);
        }
        self.visited += 1;
        self.register(node, source, guard_position)?;

        if self
            .visited
            .saturating_add(self.pending.len())
            .saturating_add(node.children.len())
            > 16384
        {
            return Ok(false);
        }
        self.pending
            .extend(node.children.iter().enumerate().map(|(index, child)| {
                let guard_position =
                    node.kind == "match_arm" && node.children.len() == 3 && index == 1;
                Visit {
                    node: child,
                    depth: depth + 1,
                    guard_position,
                }
            }));
        Ok(true)
    }

    fn register(
        &mut self,
        node: &'a HirNode,
        source: &str,
        guard_position: bool,
    ) -> Result<(), String> {
        let alias = match_guard_alias(node, guard_position)?;
        if !alias && !self.identities.insert(key(&node.hir)) {
            return Err("Mutation native inventory repeats a HIR identity".into());
        }
        if crate::source_evidence::byte_range(source, node.span)? != node.snippet {
            return Err("Mutation native node disagrees with original source bytes".into());
        }

        self.nodes
            .entry((node.kind.as_str(), node.span.start, node.span.end))
            .or_default()
            .push(node);
        Ok(())
    }
}

// This wrapper is an established frontend role, not a second native expression.
// A peeled DropTemps guard may have a different identity and is indexed normally.
fn match_guard_alias(node: &HirNode, guard_position: bool) -> Result<bool, String> {
    if node.kind != "match_guard" {
        return Ok(false);
    }
    let [guard] = node.children.as_slice() else {
        return Err("Mutation match guard wrapper has an invalid child roster".into());
    };
    if !guard_position || !guard.kind.starts_with("expr_") || !plain_guard_wrapper(node) {
        return Err("Mutation match guard wrapper has an invalid structural role".into());
    }

    let alias = node.hir == guard.hir;
    if alias && (node.span != guard.span || node.snippet != guard.snippet) {
        return Err("Mutation match guard alias disagrees with its native expression".into());
    }
    Ok(alias)
}

fn plain_guard_wrapper(node: &HirNode) -> bool {
    node.ty.is_none()
        && node.res.is_none()
        && node.dispatch.is_none()
        && node.expansion.is_none()
        && node.binding.is_none()
        && node.binding_mut.is_none()
        && node.lit.is_none()
        && node.param_index.is_none()
        && !node.is_unsafe_block
}

pub(super) fn scalar(node: &HirNode) -> bool {
    crate::frontend::normalized(node) == Some("u32")
}

pub(super) fn key(id: &HirRef) -> (u32, u32, u32) {
    (id.krate, id.owner, id.local)
}
