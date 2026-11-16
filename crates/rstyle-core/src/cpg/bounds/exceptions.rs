//! Exceptions: a node that raises one, and the regions that handle what their body raises.
//!
//! Anything in a handled region can raise, not only a node that says so, and the bound counts
//! what runs, so a path that raises partway is no longer than one that goes on. The lowering
//! lets a region's handlers start where its body ends and before every jump out of it, with
//! every counter the region names unknown: any path that raises partway and is handled is a
//! prefix of one of those paths.

use super::super::model::{AccessMode, Node, NodeKind};
use super::Collector;
use super::frame::is_pattern;
use crate::runtime_bounds::contracts::{Control, Extension};
use crate::runtime_bounds::{Action, Flow, Value};
use std::collections::BTreeSet;

type Lowered = Result<u32, String>;

impl<'a> Collector<'_, '_, 'a> {
    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// A node whose role says how it raises or handles an exception. `None` is one this lowering
    /// can't read, which stays unsupported.
    pub(super) fn control(
        &mut self,
        node: &'a Node,
        (role, control): (&Extension, Control),
        next: u32,
    ) -> Result<Option<u32>, String> {
        let children = self.facts.body.children(node)?;
        match (control, children.split_first()) {
            (Control::Throw, _) => {
                let raised = self.raised(node, next);
                self.arguments(&children, raised).map(Some)
            }
            (Control::Assert, Some((test, message))) => {
                let failed = self.raised(node, next);
                let failed = self.arguments(message, failed)?;
                let held = self.choice((next, failed), node);
                self.expr(test, held).map(Some)
            }
            (Control::Try, Some((body, rest))) => self.tried((node, role), (body, rest), next),
            (Control::With, Some(_)) => {
                let (contexts, body) = children.split_at(children.len() - 1);
                self.with(node, (body[0], contexts), next).map(Some)
            }
            _ => Ok(None),
        }
    }

    /// Where something raised at `at` goes: the handlers of the innermost region around it, or
    /// out of the function. A document whose frontend emits destructors as calls carries them
    /// on the normal path only, so there what's raised goes on as if it weren't: that path runs
    /// every destructor unwinding would, and more.
    fn raised(&mut self, at: &Node, next: u32) -> u32 {
        if self.facts.destructors_are_calls {
            return next;
        }
        match self.raises.last() {
            Some(&handlers) => handlers,
            None => self.node(Action::Nop, Flow::Return, at),
        }
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// A try: its body, then a handler or none when the body raises, then its finally blocks.
    fn tried(
        &mut self,
        (node, role): (&'a Node, &Extension),
        (body, rest): (&'a Node, &[&'a Node]),
        next: u32,
    ) -> Result<Option<u32>, String> {
        let handler = |child: &&Node| {
            child.kind == NodeKind::Match
                || child
                    .extension
                    .as_ref()
                    .is_some_and(|name| role.handlers.contains(name))
        };
        let (handlers, blocks): (Vec<&'a Node>, Vec<&'a Node>) =
            rest.iter().copied().partition(handler);
        let outer = self.raised(node, next);
        let finished = match blocks.is_empty() {
            true => None,
            false => {
                let onward = self.onward(node, &[next, outer]);
                Some(self.arguments(&blocks, onward)?)
            }
        };
        // What a handler raises, or what none handles, leaves after the finally blocks.
        let escape = self.havoc((node, rest), finished.unwrap_or(outer))?;
        let mut dispatch = escape;
        for handler in handlers.into_iter().rev() {
            let handled = self.handled(handler, (escape, finished), next)?;
            dispatch = self.choice((handled, dispatch), node);
        }
        let raise = self.havoc((node, &[body]), dispatch)?;
        self.handled(body, (raise, finished), next).map(Some)
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// A with: each context entered in order, then its body, then each exit method. What the body
    /// raises runs the exit methods too, and then goes on after the node or leaves it.
    fn with(
        &mut self,
        node: &'a Node,
        (body, contexts): (&'a Node, &[&'a Node]),
        next: u32,
    ) -> Lowered {
        let targets: Vec<_> = contexts
            .iter()
            .map(|context| self.target(context))
            .collect();
        for (context, target) in contexts.iter().zip(&targets) {
            if target.is_none() && self.writes_under(context)? {
                let reason = "A with target that writes part of a value is unmodeled";
                return Ok(self.unsupported(node, next, reason));
            }
        }
        let entered = targets.iter().filter(|target| target.is_none()).count();
        let finished = self.exits(node, entered, next);
        let raise = self.havoc((node, &[body]), finished)?;
        let entry = self.handled(body, (raise, Some(finished)), next)?;
        self.entered(node, (contexts, &targets), entry)
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// Each context's exit method, innermost first, then any of the ways out of the node.
    fn exits(&mut self, node: &'a Node, entered: usize, next: u32) -> u32 {
        let outer = self.raised(node, next);
        let mut finished = self.onward(node, &[next, outer]);
        for _ in 0..entered {
            finished = self.implicit_call(node, finished);
        }
        finished
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// Each context evaluated and entered, and each target bound to what its enter returns.
    fn entered(
        &mut self,
        node: &'a Node,
        (contexts, targets): (&[&'a Node], &[Option<bool>]),
        body: u32,
    ) -> Lowered {
        let mut entry = body;
        for (context, target) in contexts.iter().zip(targets).rev() {
            entry = match target {
                Some(true) => self.pattern_writes(context, entry)?,
                Some(false) => self.binding_write(context, entry),
                None => {
                    let enter = self.implicit_call(node, entry);
                    self.expr(context, enter)?
                }
            };
        }
        Ok(entry)
    }

    /// Whether a with's child is a target it binds: `Some(true)` a pattern, `Some(false)` a name
    /// the child writes itself, and `None` a context.
    fn target(&self, child: &'a Node) -> Option<bool> {
        if is_pattern(child.kind) {
            return Some(true);
        }
        let accesses = &self.facts.body.function.accesses;
        let written = accesses
            .iter()
            .any(|access| access.node == child.id && access.mode != AccessMode::Read);
        written.then_some(false)
    }

    /// Whether the function writes a binding anywhere in `node`.
    fn writes_under(&self, node: &'a Node) -> Result<bool, String> {
        let under = self.subtree(node)?;
        let accesses = &self.facts.body.function.accesses;
        Ok(accesses
            .iter()
            .any(|access| access.mode != AccessMode::Read && under.contains(access.node.as_str())))
    }

    fn subtree(&self, node: &'a Node) -> Result<BTreeSet<&'a str>, String> {
        let mut under = BTreeSet::new();
        let mut pending = vec![node];
        while let Some(part) = pending.pop() {
            under.insert(part.id.as_str());
            pending.extend(self.facts.body.children(part)?);
        }
        Ok(under)
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// A region whose raises go to `raise`: its normal end, and every jump out of it, may follow
    /// something it raised. With code that runs on every way out, `finished`, each way out runs
    /// it and then goes on to any of them; without, each goes on to its own destination.
    fn handled(
        &mut self,
        region: &'a Node,
        (raise, finished): (u32, Option<u32>),
        next: u32,
    ) -> Lowered {
        let end = self.choice((finished.unwrap_or(next), raise), region);
        let saved = self.loops.clone();
        let mut loops = Vec::with_capacity(saved.len());
        for &(owner, (exit, repeat)) in &saved {
            let exit = self.choice((finished.unwrap_or(exit), raise), region);
            let repeat = self.choice((finished.unwrap_or(repeat), raise), region);
            loops.push((owner, (exit, repeat)));
        }
        let returned = self.returned(region);
        let returned = self.choice((finished.unwrap_or(returned), raise), region);

        self.loops = loops;
        self.returns.push(returned);
        self.raises.push(raise);
        let entry = self.expr(region, end);
        self.raises.pop();
        self.returns.pop();
        self.loops = saved;
        entry
    }

    /// Where a return goes: the end of an inlined callable, or out of the function.
    fn returned(&mut self, at: &Node) -> u32 {
        match self.returns.last() {
            Some(&resume) => resume,
            None => self.node(Action::Nop, Flow::Return, at),
        }
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// Any of the ways out of a node around a region: `ways`, a jump out of each enclosing loop
    /// and a return. Code that runs on every way out runs once, before this, so a path that
    /// reaches it by one way can go on by another, which only adds paths.
    fn onward(&mut self, at: &Node, ways: &[u32]) -> u32 {
        let returned = self.returned(at);
        let mut destinations = ways.to_vec();
        for (_, (exit, repeat)) in &self.loops {
            destinations.extend([*exit, *repeat]);
        }
        destinations.push(returned);
        destinations.dedup();
        let mut onward = destinations.pop().unwrap_or(returned);
        for destination in destinations.into_iter().rev() {
            onward = self.choice((destination, onward), at);
        }
        onward
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// An unknown value for every counter `regions` name, as something raised partway leaves it.
    fn havoc(&mut self, (at, regions): (&'a Node, &[&'a Node]), next: u32) -> Lowered {
        let mut named = BTreeSet::new();
        for region in regions {
            for id in self.subtree(region)? {
                let node = self.facts.body.node(id)?;
                let symbol = self.facts.local(node).or_else(|| self.declared(node));
                if let Some(&variable) = symbol.and_then(|symbol| self.locals.get(symbol)) {
                    named.insert(variable);
                }
            }
        }
        let mut entry = next;
        for variable in named.into_iter().rev() {
            let action = Action::Assign {
                variable,
                value: Value::Unknown,
            };
            entry = self.node(action, Flow::Goto(entry), at);
        }
        Ok(entry)
    }
}
