//! Construction shared by every producer of `facts::Pdg`: nodes numbered as they are
//! added, control edges from the innermost enclosing construct, data edges, and the
//! canonical edge order.

use crate::facts::{EdgeKind, Pdg, PdgEdge, PdgNode};

#[derive(Debug, Default)]
pub struct GraphBuilder {
    nodes: Vec<PdgNode>,
    edges: Vec<PdgEdge>,
    /// Innermost enclosing control construct and the arm or branch being lowered.
    ctrl: Vec<(usize, usize)>,
}

impl GraphBuilder {
    /// Adds a node; inside a control construct it also gets the control edge.
    pub fn add(&mut self, node: PdgNode) -> usize {
        let index = self.nodes.len();
        self.nodes.push(node);
        if let Some(&(owner, position)) = self.ctrl.last() {
            self.edges.push(PdgEdge {
                from: owner,
                to: index,
                kind: EdgeKind::Ctrl,
                arg_pos: position,
            });
        }
        index
    }

    /// A data edge from the node producing a value, when there is one.
    pub fn link(&mut self, from: Option<usize>, to: usize, arg_pos: usize) {
        if let Some(from) = from {
            self.edges.push(PdgEdge {
                from,
                to,
                kind: EdgeKind::Data,
                arg_pos,
            });
        }
    }

    /// The graph, with its edges sorted and deduplicated.
    pub fn finish(self) -> Pdg {
        let mut edges = self.edges;
        edges.sort_by_key(|e| (e.from, e.to, e.kind, e.arg_pos));
        edges.dedup();
        Pdg {
            nodes: self.nodes,
            edges,
        }
    }
}

/// A walk that lowers a body into a `GraphBuilder`.
pub trait Lowering: Sized {
    fn graph(&mut self) -> &mut GraphBuilder;

    /// Runs `lower` with `owner`'s arm `position` as the enclosing control construct.
    fn under<R>(&mut self, owner: usize, position: usize, lower: impl FnOnce(&mut Self) -> R) -> R {
        self.graph().ctrl.push((owner, position));
        let result = lower(self);
        self.graph().ctrl.pop();
        result
    }
}
