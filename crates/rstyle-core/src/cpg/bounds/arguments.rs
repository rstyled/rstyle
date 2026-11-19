//! Summary arguments follow formal positions; evaluating them follows source order.

use super::super::model::{Callable, Dispatch, Node, RefStatus};
use super::Collector;
use crate::advisory::runtime_bounds::MAX_VARIABLES;
use crate::runtime_bounds::Argument;
use std::collections::BTreeSet;

impl<'a> Collector<'_, '_, 'a> {
    /// Missing, unpacked or ambiguous bindings supply no size information. In particular,
    /// an omitted default is not the next written argument, nor a zero-valued argument.
    pub(super) fn summary_arguments(&mut self, call: &'a Node) -> Result<Vec<Argument>, String> {
        let passed = self.facts.body.arguments(call)?;
        // An allocator's written operands include a compile-time type its callable doesn't
        // list, so its call keeps every written slot, as one whose callable isn't known does.
        let layout = match self.allocation_operands(call)? {
            Some(_) => None,
            None => self.argument_layout(call),
        };
        let count = layout.map_or(passed.len(), |(count, _)| count);
        if count > MAX_VARIABLES || !self.charge(count) {
            return Ok(Vec::new());
        }
        let mut inputs = vec![Argument::Opaque; count];
        let Some((_, offset)) = layout else {
            return Ok(inputs);
        };
        let bindings = self
            .facts
            .reference(call)
            .and_then(|reference| reference.arguments.as_ref());
        if bindings.is_some_and(|bindings| bindings.iter().any(|binding| binding.spread.is_some()))
        {
            return Ok(inputs);
        }
        let positions: Option<Vec<usize>> = passed
            .iter()
            .map(|argument| {
                argument
                    .position?
                    .checked_add(offset)
                    .filter(|&at| at < count)
            })
            .collect();
        let Some(positions) = positions else {
            return Ok(inputs);
        };
        let mut occupied = BTreeSet::new();
        if positions.iter().any(|position| !occupied.insert(*position)) {
            return Ok(inputs);
        }
        for (argument, position) in passed.iter().zip(positions) {
            inputs[position] = self.argument(argument.value)?;
        }
        Ok(inputs)
    }

    /// A positional call number identifies a formal only for a fixed, known interface.
    /// Variadic elements can otherwise occupy the numbers of later formal parameters.
    fn argument_layout(&self, call: &'a Node) -> Option<(usize, usize)> {
        let callable = self.argument_callable(call)?;
        let reference = self.facts.reference(call)?;
        // A call read in method form without a recorded dispatch holds its receiver first, where
        // its written arguments leave it out; whether the frontend numbered them from the
        // receiver isn't known, so none binds a formal.
        if reference.dispatch.is_none() && reference.reason.is_some() {
            return None;
        }
        let method = matches!(
            reference.dispatch,
            Some(Dispatch::Method | Dispatch::Dynamic)
        );
        if method && callable.receiver.is_none() {
            return None;
        }
        let callee = self.facts.body.child(call, 0).ok()?;
        let made = self.facts.type_call(call, callee).is_some();
        let receiver = usize::from(callable.receiver.is_some());
        let count = callable.parameters.len().checked_add(receiver)?;
        Some((count, usize::from(made) * receiver))
    }

    /// Use the implementation that summary composition selects, rather than an interface's
    /// declaration, without changing which call targets the lowering can resolve.
    fn argument_callable(&self, call: &Node) -> Option<&'a Callable> {
        let reference = self.facts.reference(call)?;
        let declared = self.facts.declared(reference)?;
        let definition = if self.facts.interface_member(declared) {
            if reference.status == RefStatus::Dependent {
                return None;
            }
            self.facts
                .tables
                .named_definition(reference.selected.as_ref()?.id.as_deref())?
        } else {
            declared
        };
        let callable = definition.callable.as_ref()?;
        // A positive fact on either definition overrides an incomplete callable roster.
        let mut modifiers = declared
            .modifiers
            .iter()
            .chain(&definition.modifiers)
            .chain(&callable.modifiers);
        (!modifiers.any(|modifier| modifier == "variadic")).then_some(callable)
    }
}
