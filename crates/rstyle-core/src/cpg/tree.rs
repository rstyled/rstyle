//! Per-function and per-unit indexes, with the tree checks the derivations rely on.

use super::model::{
    Definition, FileSpan, Function, Node, NodeKind, Origin, Ref, Symbol, Type, Unit,
};
use crate::wording::Wording;
use std::collections::{BTreeMap, BTreeSet};

/// Unit tables by ID.
pub struct Tables<'a> {
    definitions: BTreeMap<&'a str, &'a Definition>,
    types: BTreeMap<&'a str, &'a Type>,
    unbounded_integers: bool,
    wording: Wording,
}

impl<'a> Tables<'a> {
    pub fn new(unit: &'a Unit) -> Result<Self, String> {
        Ok(Self {
            definitions: index("definition", &unit.definitions, |d| &d.id)?,
            types: index("type", &unit.types, |t| &t.id)?,
            unbounded_integers: unit.semantics.integer_overflow == "unbounded",
            wording: Wording::of(&unit.language.name),
        })
    }

    /// The terms the unit's messages use.
    pub fn wording(&self) -> Wording {
        self.wording
    }

    /// Whether the unit's integers have no width, so an integer type needs no `bits`.
    pub fn unbounded_integers(&self) -> bool {
        self.unbounded_integers
    }

    pub fn definition(&self, id: &str) -> Result<&'a Definition, String> {
        lookup(&self.definitions, id, || format!("Unknown definition {id}"))
    }

    /// The definition something optionally names, when it names one the unit holds.
    pub fn named_definition(&self, id: Option<&str>) -> Option<&'a Definition> {
        self.definition(id?).ok()
    }

    pub fn ty(&self, id: &str) -> Result<&'a Type, String> {
        lookup(&self.types, id, || format!("Unknown type {id}"))
    }
}

/// A record by ID, or the error `missing` describes.
pub fn lookup<V: Copy>(
    table: &BTreeMap<&str, V>,
    id: &str,
    missing: impl FnOnce() -> String,
) -> Result<V, String> {
    table.get(id).copied().ok_or_else(missing)
}

fn index<'a, T>(
    kind: &str,
    records: &'a [T],
    id: impl Fn(&'a T) -> &'a String,
) -> Result<BTreeMap<&'a str, &'a T>, String> {
    let mut table = BTreeMap::new();
    for record in records {
        if table.insert(id(record).as_str(), record).is_some() {
            return Err(format!("Duplicate {kind} ID {}", id(record)));
        }
    }
    Ok(table)
}

/// One function body: its nodes form a tree under the root.
pub struct Body<'a> {
    pub function: &'a Function,
    nodes: BTreeMap<&'a str, &'a Node>,
    symbols: BTreeMap<&'a str, &'a Symbol>,
    refs: BTreeMap<&'a str, &'a Ref>,
}

impl<'a> Body<'a> {
    pub fn new(function: &'a Function) -> Result<Self, String> {
        let body = Self {
            function,
            nodes: index("node", &function.nodes, |n| &n.id)?,
            symbols: index("symbol", &function.symbols, |s| &s.id)?,
            refs: index("ref", &function.refs, |r| &r.node)?,
        };
        body.check_tree()?;
        Ok(body)
    }

    fn check_tree(&self) -> Result<(), String> {
        let mut parented = BTreeSet::new();
        for node in self.nodes.values() {
            for child in &node.children {
                self.node(child)?;
                if !parented.insert(child.as_str()) {
                    return Err(format!("Node {child} has two parents"));
                }
            }
        }
        if parented.contains(self.function.root.as_str()) {
            return Err(format!("Function {}: root has a parent", self.function.id));
        }
        self.node(&self.function.root).map(|_| ())
    }

    pub fn node(&self, id: &str) -> Result<&'a Node, String> {
        let unknown = || format!("Function {}: unknown node {id}", self.function.id);
        lookup(&self.nodes, id, unknown)
    }

    pub fn symbol(&self, id: &str) -> Result<&'a Symbol, String> {
        let unknown = || format!("Function {}: unknown symbol {id}", self.function.id);
        lookup(&self.symbols, id, unknown)
    }

    pub fn reference(&self, node: &str) -> Option<&'a Ref> {
        self.refs.get(node).copied()
    }

    pub fn root(&self) -> Result<&'a Node, String> {
        self.node(&self.function.root)
    }

    pub fn children(&self, node: &Node) -> Result<Vec<&'a Node>, String> {
        node.children.iter().map(|child| self.node(child)).collect()
    }

    pub fn child(&self, node: &Node, index: usize) -> Result<&'a Node, String> {
        let id = node
            .children
            .get(index)
            .ok_or_else(|| format!("Node {} has no child {index}", node.id))?;
        self.node(id)
    }

    /// An assignment whose target a destructuring rebuilt from its pattern.
    pub fn destructures(&self, assignment: &Node) -> Result<bool, String> {
        let target = self.child(assignment, 0)?;
        Ok(matches!(
            target.kind,
            NodeKind::Tuple
                | NodeKind::Array
                | NodeKind::RecordConstruction
                | NodeKind::PatternWildcard
        ))
    }

    /// The places a destructuring target names, left to right.
    pub fn places(&self, target: &'a Node) -> Result<Vec<&'a Node>, String> {
        let mut found = Vec::new();
        self.collect_places(target, &mut found)?;
        Ok(found)
    }

    fn collect_places(&self, target: &'a Node, found: &mut Vec<&'a Node>) -> Result<(), String> {
        match target.kind {
            NodeKind::Tuple | NodeKind::Array | NodeKind::RecordConstruction => {
                for part in self.children(target)? {
                    self.collect_places(part, found)?;
                }
            }
            NodeKind::PatternWildcard => {}
            _ => found.push(target),
        }
        Ok(())
    }

    /// Children that are source nodes or synthetic wrappers of one (the profile's child order).
    pub fn source_children(&self, node: &Node) -> Result<Vec<&'a Node>, String> {
        let mut sources = Vec::new();
        for child in self.children(node)? {
            if self.has_source(child)? {
                sources.push(child);
            }
        }
        Ok(sources)
    }

    pub fn has_source(&self, node: &Node) -> Result<bool, String> {
        if !node.synthetic {
            return Ok(true);
        }
        for child in self.children(node)? {
            if self.has_source(child)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// What the language runs as a scope ends, such as a destructor, wherever the scope was written.
pub fn scope_end(node: &Node) -> bool {
    node.synthetic
        && node.provenance.origin == Origin::Desugaring
        && node.provenance.construct.as_deref() == Some("scope")
}

/// Macro-expanded or desugared: code the developer did not write at this node.
pub fn generated(node: &Node) -> bool {
    matches!(
        node.provenance.origin,
        Origin::MacroExpansion | Origin::Desugaring
    )
}

/// Where a node is reported: its own span, or its call site when generated or spanless.
pub fn anchor(node: &Node) -> Result<&FileSpan, String> {
    let call_site = node.provenance.call_site.as_ref();
    let own = node.span.as_ref();
    let chosen = if generated(node) {
        call_site.or(own)
    } else {
        own.or(call_site)
    };
    chosen.ok_or_else(|| format!("Node {} has neither a span nor a call site", node.id))
}

/// The line of a node's location.
pub fn line(sources: &super::source::Sources<'_>, node: &Node) -> Result<usize, String> {
    Ok(sources.location(anchor(node)?)?.0)
}

pub fn is_kind(node: &Node, kinds: &[NodeKind]) -> bool {
    kinds.contains(&node.kind)
}
