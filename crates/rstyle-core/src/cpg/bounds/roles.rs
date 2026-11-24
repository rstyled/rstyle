//! Control roles that aren't about exceptions: a call run later than where it's written, a
//! removal, and a chain of comparisons.

use super::super::model::{Node, NodeKind, TypeKind};
use super::Collector;

type Lowered = Result<u32, String>;

impl<'a> Collector<'_, '_, 'a> {
    /// A call that runs later than where it's written, when the function exits or on a thread of
    /// its own, with its callee and arguments evaluated here. It's lowered here, so it counts once
    /// each time the node runs. A closure in it reads variables when the call runs, not here, so a
    /// node that holds one, written there or named by a local the function binds it to, is
    /// unsupported.
    pub(super) fn deferred(&mut self, node: &'a Node, calls: &[&'a Node], next: u32) -> Lowered {
        for call in calls {
            for id in self.subtree(call)? {
                let part = self.facts.body.node(id)?;
                let named = self
                    .facts
                    .local(part)
                    .is_some_and(|symbol| self.closures.contains_key(symbol));
                if part.kind == NodeKind::Closure || named {
                    let reason = "A closure that runs later reads values that can change first";
                    return Ok(self.unsupported(node, next, reason));
                }
            }
        }
        self.arguments(calls, next)
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// Each target removed in order: a name is written, and an item or an attribute is removed by
    /// a method its container defines, an unresolved call once the container and the key are
    /// evaluated.
    pub(super) fn deleted(&mut self, node: &'a Node, targets: &[&'a Node], next: u32) -> Lowered {
        let mut entry = next;
        for target in targets.iter().rev() {
            entry = match target.kind {
                NodeKind::Identifier => self.binding_write(target, entry),
                NodeKind::IndexAccess | NodeKind::FieldAccess => {
                    let removed = self.implicit_call(node, entry);
                    let parts = self.facts.body.children(target)?;
                    self.arguments(&parts, removed)?
                }
                _ => {
                    let reason =
                        "Removing anything but a name, an item or an attribute is unmodeled";
                    return Ok(self.unsupported(node, next, reason));
                }
            };
        }
        Ok(entry)
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// A chain of comparisons: its first two operands, then each next one only while every
    /// comparison so far held. A comparison of operands not both of primitive types runs a method
    /// they define, an unresolved call.
    pub(super) fn chained(
        &mut self,
        node: &'a Node,
        (first, rest): (&'a Node, &[&'a Node]),
        next: u32,
    ) -> Lowered {
        let mut entry = next;
        for (index, operand) in rest.iter().enumerate().rev() {
            let previous = match index {
                0 => first,
                _ => rest[index - 1],
            };
            if !self.compared_natively(previous) || !self.compared_natively(operand) {
                entry = self.implicit_call(node, entry);
            }
            entry = self.expr(operand, entry)?;
            if index > 0 {
                entry = self.choice((entry, next), node);
            }
        }
        self.expr(first, entry)
    }

    /// Whether a value is of a type whose comparisons run only the language's own code.
    fn compared_natively(&self, value: &Node) -> bool {
        matches!(
            self.facts.ty(value).map(|ty| ty.kind),
            Some(
                TypeKind::Integer
                    | TypeKind::Float
                    | TypeKind::Bool
                    | TypeKind::Char
                    | TypeKind::String
            )
        )
    }
}
