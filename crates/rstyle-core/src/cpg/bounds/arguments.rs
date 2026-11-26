//! Summary arguments follow the callee's own parameter positions, a receiver first when it takes
//! one; evaluating them follows source order.

use super::super::model::{Callable, Dispatch, Node, RefStatus};
use super::super::tree::Argument as TreeArgument;
use super::Collector;
use crate::advisory::runtime_bounds::MAX_VARIABLES;
use crate::runtime_bounds::Argument;
use std::collections::BTreeSet;

/// How a call's arguments can bind its callee's parameters.
#[derive(Clone, Copy)]
struct Layout {
    /// The callee's parameters, its receiver included.
    count: usize,
    /// The first parameter an argument can bind: 1 where the call makes the receiver itself.
    first: usize,
    /// Whether the call passes a receiver the callee doesn't take, as a static method called
    /// through an instance does.
    unpassed_receiver: bool,
}

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
        let count = layout.map_or(passed.len(), |layout| layout.count);
        if count > MAX_VARIABLES || !self.charge(count) {
            return Ok(Vec::new());
        }
        let mut inputs = vec![Argument::Opaque; count];
        let Some(layout) = layout else {
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
        let Some(placed) = self.bound_arguments(call, &passed, layout) else {
            return Ok(inputs);
        };
        for (argument, position) in placed {
            inputs[position] = self.argument(argument.value)?;
        }
        Ok(inputs)
    }

    /// Each argument that binds a parameter, at that parameter, or nothing when one's position
    /// isn't known or doesn't fit the callee. A receiver bound by binding nothing is left out: a
    /// method's that a class object reads, and a static method's, which no number may give.
    /// Where the call makes the receiver itself, no argument may bind it. A document that numbers
    /// arguments by the call rather than by the callee breaks one of those, so it binds none.
    fn bound_arguments<'p>(
        &self,
        call: &Node,
        passed: &'p [TreeArgument<'a>],
        layout: Layout,
    ) -> Option<Vec<(&'p TreeArgument<'a>, usize)>> {
        let method = self.facts.reference(call).is_some_and(|reference| {
            matches!(
                reference.dispatch,
                Some(Dispatch::Method | Dispatch::Dynamic)
            )
        });
        let bound = self
            .facts
            .reference(call)
            .is_some_and(|reference| reference.arguments.is_some());
        let receiver = passed.first().filter(|_| method);
        let unbound_receiver = receiver.is_some_and(|it| bound && it.position.is_none());
        if layout.unpassed_receiver && !unbound_receiver {
            return None;
        }
        let binding = &passed[usize::from(unbound_receiver)..];

        let mut occupied = BTreeSet::new();
        let mut placed = Vec::with_capacity(binding.len());
        for argument in binding {
            let position = argument.position?;
            let fits = (layout.first..layout.count).contains(&position);
            if !fits || !occupied.insert(position) {
                return None;
            }
            placed.push((argument, position));
        }
        Some(placed)
    }

    /// A positional call number identifies a formal only for a fixed, known interface.
    /// Variadic elements can otherwise occupy the numbers of later formal parameters.
    fn argument_layout(&self, call: &'a Node) -> Option<Layout> {
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
        let callee = self.facts.body.child(call, 0).ok()?;
        let made = !method && self.facts.type_call(call, callee).is_some();
        let receiver = callable.receiver.is_some();
        let count = callable
            .parameters
            .len()
            .checked_add(usize::from(receiver))?;
        Some(Layout {
            count,
            first: usize::from(made && receiver),
            unpassed_receiver: method && !receiver,
        })
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
