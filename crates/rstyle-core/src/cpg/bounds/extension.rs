//! Nodes of a language's own registered extensions, lowered by the role library contract data
//! gives each one, so core needs no table of any language's syntax.

use super::super::model::{Node, NodeKind, TargetKind};
use super::Collector;
use super::frame::is_pattern;
use crate::runtime_bounds::contracts::{Contracts, Implicit};
use crate::runtime_bounds::{Action, Callee, Flow, Value};

type Lowered = Result<u32, String>;

impl<'a> Collector<'_, '_, 'a> {
    /// The node's children once, in the order the profile writes them, with each pattern among
    /// them a binding it writes; then, when its role says so, the operation it runs with no callee
    /// the document names, as an unresolved call. `None` is an extension without a role.
    pub(super) fn extension(&mut self, node: &'a Node, next: u32) -> Result<Option<u32>, String> {
        let Some(role) = node
            .extension
            .as_deref()
            .and_then(|name| Contracts::embedded().extension(name))
        else {
            return Ok(None);
        };
        let operates = match role.implicit {
            Implicit::Never => false,
            Implicit::Always => true,
            Implicit::WithoutMember => node.member.is_none(),
        };
        let mut entry = match operates {
            true => self.implicit_call(node, next),
            false => next,
        };
        for child in self.facts.body.children(node)?.into_iter().rev() {
            entry = match is_pattern(child.kind) {
                true => self.pattern_writes(child, entry)?,
                false => self.expr(child, entry)?,
            };
        }
        Ok(Some(entry))
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    fn implicit_call(&mut self, node: &'a Node, next: u32) -> u32 {
        let action = Action::Call {
            callee: node.extension.clone().unwrap_or_default(),
            target: Callee::Unresolved,
            arguments: Vec::new(),
            may_modify: false,
            operation: None,
        };
        self.node(action, Flow::Goto(next), node)
    }

    /// What a pattern writes: each binding it holds, new or not, and the places it holds besides.
    pub(super) fn pattern_writes(&mut self, pattern: &'a Node, next: u32) -> Lowered {
        let mut entry = self.declaration_writes(pattern, next)?;
        let mut pending = vec![pattern];
        while let Some(part) = pending.pop() {
            if !is_pattern(part.kind) {
                continue;
            }
            pending.extend(self.facts.body.children(part)?);
            if part.kind == NodeKind::PatternBinding {
                entry = self.binding_write(part, entry);
            }
        }
        Ok(entry)
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// A binding pattern's write of what it binds: an unknown value for a counter.
    fn binding_write(&mut self, binding: &'a Node, next: u32) -> u32 {
        let Some(symbol) = self.bound_symbol(binding) else {
            return next;
        };
        if let Some(&variable) = self.locals.get(symbol) {
            let action = Action::Assign {
                variable,
                value: Value::Unknown,
            };
            return self.node(action, Flow::Goto(next), binding);
        }
        let root = format!("{symbol}:");
        if self.extents.keys().any(|key| key.starts_with(&root)) {
            return self.unsupported(binding, next, "Declaration writes an extent source");
        }
        next
    }

    /// The symbol a binding declares, or the one it binds again in a language whose names
    /// belong to the whole function.
    fn bound_symbol(&self, binding: &'a Node) -> Option<&'a str> {
        self.declared(binding).or_else(|| {
            let target = self.facts.reference(binding)?.target.as_ref()?;
            (target.kind == TargetKind::Symbol)
                .then_some(target.id.as_deref())
                .flatten()
        })
    }
}
