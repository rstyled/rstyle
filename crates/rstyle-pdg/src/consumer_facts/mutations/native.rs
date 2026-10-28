use crate::{BindingFact, ByteSpan, HirFnFacts, HirNode, HirRef};
use std::collections::{BTreeMap, BTreeSet};

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
        match self
            .nodes
            .get(&(kind, span.start, span.end))
            .map(Vec::as_slice)
        {
            None => Ok(None),
            Some([node]) if node.expansion.is_none() => Ok(Some(*node)),
            Some([_]) => Ok(None),
            Some(_) => Err("Mutation source site has ambiguous native correspondence".into()),
        }
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

struct Inventory<'a> {
    nodes: Nodes<'a>,
    identities: BTreeSet<(u32, u32, u32)>,
    pending: Vec<(&'a HirNode, usize)>,
}

impl<'a> Inventory<'a> {
    fn new(facts: &'a HirFnFacts) -> Self {
        let pending = facts
            .signature
            .params
            .iter()
            .map(|parameter| (&parameter.pat, 0))
            .chain(std::iter::once((&facts.body, 0)))
            .collect();
        Self {
            nodes: BTreeMap::new(),
            identities: BTreeSet::new(),
            pending,
        }
    }

    fn collect(mut self, source: &str) -> Result<Option<Nodes<'a>>, String> {
        while let Some((node, depth)) = self.pending.pop() {
            if !self.visit(node, depth, source)? {
                return Ok(None);
            }
        }
        Ok(Some(self.nodes))
    }

    fn visit(&mut self, node: &'a HirNode, depth: usize, source: &str) -> Result<bool, String> {
        if self.identities.len() >= 16384 || depth > 128 {
            return Ok(false);
        }
        if !self.identities.insert(key(&node.hir)) {
            return Err("Mutation native inventory repeats a HIR identity".into());
        }
        if crate::source_evidence::byte_range(source, node.span)? != node.snippet {
            return Err("Mutation native node disagrees with original source bytes".into());
        }

        self.nodes
            .entry((node.kind.as_str(), node.span.start, node.span.end))
            .or_default()
            .push(node);
        if self
            .identities
            .len()
            .saturating_add(self.pending.len())
            .saturating_add(node.children.len())
            > 16384
        {
            return Ok(false);
        }
        self.pending
            .extend(node.children.iter().map(|child| (child, depth + 1)));
        Ok(true)
    }
}

pub(super) fn scalar(node: &HirNode) -> bool {
    crate::frontend::normalized(node) == Some("u32")
}

pub(super) fn key(id: &HirRef) -> (u32, u32, u32) {
    (id.krate, id.owner, id.local)
}
