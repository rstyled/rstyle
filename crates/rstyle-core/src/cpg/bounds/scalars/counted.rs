//! Counted Go loops with scalar-local state and a known condition on every visit.
use super::{Node, NodeKind, Scalar, TypeKind, Walk};

impl<'a> Walk<'_, '_, 'a> {
    fn scalar_loop_body(&mut self, node: &'a Node, depth: usize) -> Option<bool> {
        self.enter(node, depth)?;
        // A fresh iteration variable may only share its storage with the previous visit
        // when an outside observer cannot retain either address.
        let supported = match node.kind {
            NodeKind::PatternBinding => matches!(
                self.facts.ty(node)?.kind,
                TypeKind::Integer | TypeKind::Bool | TypeKind::Float
            ),
            NodeKind::Block
            | NodeKind::ExpressionStatement
            | NodeKind::LocalDeclaration
            | NodeKind::CompoundAssignment
            | NodeKind::Assignment
            | NodeKind::Operator
            | NodeKind::Literal
            | NodeKind::Identifier
            | NodeKind::If
            | NodeKind::Return => true,
            _ => false,
        };
        if !supported {
            return Some(false);
        }
        for id in &node.children {
            if !self.scalar_loop_body(self.facts.body.node(id).ok()?, depth + 1)? {
                return Some(false);
            }
        }
        Some(true)
    }

    pub(super) fn counted_loop(&mut self, node: &'a Node, depth: usize) -> Option<()> {
        let cond = self.child(node, 0)?;
        let region = self.child(node, 1)?;
        if region.kind != NodeKind::Block
            || region.label.is_some()
            || !self.scalar_loop_body(cond, depth + 1)?
            || !self.scalar_loop_body(region, depth + 1)?
        {
            return None;
        }
        let (body, post) = if region.original_kind == self.profile.counted_loop_header {
            // The syntax tree keeps the post before the body, in source order.
            // The post executes after each completed body.
            if region.children.len() != 2 {
                return None;
            }
            (self.child(region, 1)?, Some(self.child(region, 0)?))
        } else {
            (region, None)
        };
        let previous = std::mem::take(&mut self.values);
        let outcome = self.run_loop((cond, body, post), depth);
        self.values = previous;
        outcome
    }

    fn run_loop(
        &mut self,
        (cond, body, post): (&'a Node, &'a Node, Option<&'a Node>),
        depth: usize,
    ) -> Option<()> {
        loop {
            match self.value(cond, depth + 1)? {
                Scalar::Bool(false) => return Some(()),
                Scalar::Bool(true) => {
                    self.statement(body, depth + 1)?;
                    if let Some(p) = post {
                        self.statement(p, depth + 1)?;
                    }
                }
                _ => return None,
            }
        }
    }
}
