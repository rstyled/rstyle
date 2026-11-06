//! Resolved standard iterator chains lower to synthetic scans with inlined callbacks.
//!
//! A chain is modeled only when its source is a stable slice or `Vec` traversal or a
//! primitive `Range`, and every stage is a resolved member of the standard iteration
//! interface with a stage or consumer role in the library contracts. Each callback
//! runs at most once per pulled source item; filters, skips and short-circuiting
//! consumers become nondeterministic exits.

use super::super::model::{DefinitionKind, Node, NodeKind, TargetKind, Type, TypeKind};
use super::Collector;
use super::extent::{has_role, is_method};
use crate::runtime_bounds::contracts::{
    ConsumerEffect, Contracts, ExtentRole, Iteration, StageEffect,
};
use crate::runtime_bounds::{Action, Callee, Condition, Flow, Integer, Value};

const MAX_STAGES: usize = 32;
const CHAIN_EVIDENCE: &str = "live typed HIR; resolved standard iterator chain over a stable extent or primitive range; callbacks inlined at each invocation";

#[derive(Clone, Copy)]
pub(super) enum Callback<'a> {
    Closure(&'a Node),
    Function(&'a str),
}

#[derive(Clone, Copy)]
enum Stage<'a> {
    Pass,
    Map(Callback<'a>),
    Filter(Callback<'a>),
    Skip,
    Take,
    TakeWhile(Callback<'a>),
    FlatMap(&'a Node),
}

#[derive(Clone, Copy)]
enum Consumer<'a> {
    Drain,
    Each(Callback<'a>),
    Search(Callback<'a>),
}

struct Chain<'a> {
    root: &'a Node,
    limit: Value,
    bits: u8,
    stages: Vec<Stage<'a>>,
    arguments: Vec<&'a Node>,
    standard: bool,
}

#[derive(Clone, Copy)]
struct Tail<'a> {
    consumer: Consumer<'a>,
    targets: Targets,
    at: &'a Node,
}

#[derive(Clone, Copy)]
struct Targets {
    skip: u32,
    stop: u32,
}

#[derive(Clone, Copy)]
struct Counter<'a> {
    index: u32,
    limit: u32,
    header: u32,
    at: &'a Node,
}

#[derive(Clone, Copy)]
struct Step<'a> {
    callback: Option<Callback<'a>>,
    alternative: Option<u32>,
}

type Lowered = Result<u32, String>;

impl<'a> Collector<'_, '_, 'a> {
    /// The contract key of a resolved member of the standard iteration interface.
    pub(super) fn iterator_method(&self, call: &Node) -> Option<&'a str> {
        if !is_method(self.facts, call) {
            return None;
        }
        let reference = self.facts.reference(call)?;
        let declared = self.facts.declared(reference)?;
        declared
            .iteration
            .then_some(declared.operation.as_deref())
            .flatten()
    }

    /// `None` keeps ordinary call lowering, which rejects any unmodeled callback.
    pub(super) fn iterator_chain(
        &mut self,
        expression: &'a Node,
        next: u32,
    ) -> Result<Option<u32>, String> {
        let children = self.facts.body.children(expression)?;
        let Some((&receiver, arguments)) = children.split_first() else {
            return Ok(None);
        };
        let Some((consumer, contract)) = self.consumer(expression, arguments)? else {
            return Ok(None);
        };
        let Some(mut chain) = self.chain(receiver)? else {
            return Ok(None);
        };
        if !chain.standard || !self.standard_dispatch(expression, receiver, &contract.dispatch)? {
            let reason = "Iterator chain dispatches to a non-standard trait implementation";
            return Ok(Some(self.unsupported(expression, next, reason)));
        }
        // Every argument but the inlined callback is evaluated, so other closures stay visible.
        chain
            .arguments
            .extend(evaluated(arguments, contract.callback));
        let tail = Tail {
            consumer,
            targets: Targets {
                skip: next,
                stop: next,
            },
            at: expression,
        };
        let scan = self.scan_chain(&chain, &[], tail)?;

        Ok(Some(self.chain_inputs(&chain, scan)?))
    }

    // Source places and stage arguments are evaluated once, before the first pull.
    fn chain_inputs(&mut self, chain: &Chain<'a>, scan: u32) -> Lowered {
        let mut arguments = scan;
        for argument in chain.arguments.iter().rev() {
            arguments = self.expr(argument, arguments)?;
        }
        if is_method(self.facts, chain.root) && chain.root.children.len() == 1 {
            return self.expr(self.facts.body.child(chain.root, 0)?, arguments);
        }
        self.expr(chain.root, arguments)
    }

    /// Iteration calls whose callable arguments cannot all be resolved stay unknown.
    pub(super) fn unresolved_callback(&self, expression: &'a Node) -> Result<bool, String> {
        if self.iterator_method(expression).is_none() {
            return Ok(false);
        }
        let arguments = &self.facts.body.children(expression)?[1..];
        Ok(arguments.iter().any(|argument| {
            self.facts
                .adjusted(argument)
                .is_some_and(|ty| self.facts.any_part(ty, 64, &Self::callable))
        }))
    }

    // Any callable component, such as a boxed `dyn Fn`, may be invoked per item.
    fn callable(ty: &Type) -> bool {
        matches!(
            ty.kind,
            TypeKind::Closure
                | TypeKind::Function
                | TypeKind::Parameter
                | TypeKind::Interface
                | TypeKind::Opaque
        )
    }

    fn consumer(
        &self,
        expression: &'a Node,
        arguments: &[&'a Node],
    ) -> Result<Option<(Consumer<'a>, &'static Iteration<ConsumerEffect>)>, String> {
        let Some(key) = self.iterator_method(expression) else {
            return Ok(None);
        };
        let Some(contract) = Contracts::embedded().consumer(key) else {
            return Ok(None);
        };
        let Some(callback) = self.role_callback(contract, arguments) else {
            return Ok(None);
        };

        let consumer = match (contract.effect, callback) {
            (ConsumerEffect::Drain, _) => Consumer::Drain,
            (ConsumerEffect::Each, Some(callback)) => Consumer::Each(callback),
            (ConsumerEffect::Search, Some(callback)) => Consumer::Search(callback),
            _ => return Ok(None),
        };
        Ok(Some((consumer, contract)))
    }

    /// The role's resolved callback; `None` when the arity differs or the callback is
    /// unresolved, so ordinary call lowering keeps the call visible.
    fn role_callback<E>(
        &self,
        contract: &Iteration<E>,
        arguments: &[&'a Node],
    ) -> Option<Option<Callback<'a>>> {
        (arguments.len() == contract.arity as usize).then_some(())?;
        match contract.callback {
            Some(index) => self.callback(arguments.get(index as usize)?).map(Some),
            None => Some(None),
        }
    }

    fn chain(&self, mut receiver: &'a Node) -> Result<Option<Chain<'a>>, String> {
        let mut links = Vec::new();
        while let Some(key) = self.iterator_method(receiver) {
            let children = self.facts.body.children(receiver)?;
            let (&inner, items) = children
                .split_first()
                .ok_or_else(|| format!("Method call {} has no receiver", receiver.id))?;
            if links.len() >= MAX_STAGES || self.facts.adjusted_of(inner) {
                return Ok(None);
            }
            let Some((stage, contract)) = self.stage(key, items) else {
                return Ok(None);
            };
            let standard = self.standard_dispatch(receiver, inner, &contract.dispatch)?;
            links.push((
                stage,
                evaluated(items, contract.callback).collect::<Vec<_>>(),
                standard,
            ));
            receiver = inner;
        }
        // Links were walked outermost first; the source's stage runs first.
        let Some((limit, bits)) = self.source(receiver)? else {
            return Ok(None);
        };
        Ok(Some(Chain {
            root: receiver,
            limit,
            bits,
            stages: links.iter().rev().map(|link| link.0).collect(),
            arguments: links.iter().rev().flat_map(|link| link.1.clone()).collect(),
            standard: links.iter().all(|link| link.2),
        }))
    }

    fn stage(
        &self,
        key: &str,
        items: &[&'a Node],
    ) -> Option<(Stage<'a>, &'static Iteration<StageEffect>)> {
        let contract = Contracts::embedded().stage(key)?;
        let callback = self.role_callback(contract, items)?;
        let stage = match contract.effect {
            StageEffect::Pass => Stage::Pass,
            StageEffect::Map => Stage::Map(callback?),
            StageEffect::Filter => Stage::Filter(callback?),
            StageEffect::Skip => Stage::Skip,
            StageEffect::Take => Stage::Take,
            StageEffect::TakeWhile => Stage::TakeWhile(callback?),
            StageEffect::FlatMap => match callback? {
                Callback::Closure(closure) => Stage::FlatMap(closure),
                Callback::Function(_) => return None,
            },
        };
        Some((stage, contract))
    }

    // Slice and `Vec` traversals yield at most their stable length.
    fn source(&self, root: &'a Node) -> Result<Option<(Value, u8)>, String> {
        if root.kind == NodeKind::RecordConstruction {
            return self.range_source(root);
        }
        if !is_method(self.facts, root) || root.children.len() != 1 {
            return Ok(None);
        }
        if !has_role(self.facts, root, ExtentRole::Traversal) {
            return Ok(None);
        }
        let Some(place) = self.place(self.facts.body.child(root, 0)?)? else {
            return Ok(None);
        };

        Ok(self
            .extents
            .get(&place.key())
            .map(|label| (Value::Input(label.clone()), self.facts.pointer_bits)))
    }

    // A primitive `start..end` yields at most `end` items, whatever its start.
    fn range_source(&self, root: &'a Node) -> Result<Option<(Value, u8)>, String> {
        let Some((_, end, bits)) = self.range_parts(root)? else {
            return Ok(None);
        };
        let limit = self.value(end);
        Ok((!matches!(limit, Value::Unknown)).then_some((limit, bits)))
    }

    pub(super) fn callback(&self, argument: &'a Node) -> Option<Callback<'a>> {
        match argument.kind {
            NodeKind::Closure if !argument.modifiers.iter().any(|m| m == "coroutine") => {
                Some(Callback::Closure(argument))
            }
            NodeKind::Identifier => {
                if let Some(symbol) = self.facts.local(argument) {
                    return self.closures.get(symbol).copied().map(Callback::Closure);
                }
                match self.facts.resolved(argument) {
                    Some((TargetKind::Definition, definition))
                        if matches!(
                            definition.kind,
                            DefinitionKind::Function | DefinitionKind::Method
                        ) =>
                    {
                        Some(Callback::Function(definition.id.as_str()))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// Scans `chain`, then `rest` per item; `tail.targets` are the scan exit and chain stop.
    fn scan_chain(&mut self, chain: &Chain<'a>, rest: &[Stage<'a>], tail: Tail<'a>) -> Lowered {
        let integer = Integer::unsigned(chain.bits);
        let index = self.variable("chain-index".into(), Value::Unknown, integer);
        let limit = self.variable("chain-limit".into(), Value::Unknown, integer);
        let header = self.chain_header(tail.at, tail.targets.skip);
        let counter = Counter {
            index,
            limit,
            header,
            at: tail.at,
        };
        let stages: Vec<_> = chain.stages.iter().chain(rest).copied().collect();
        let body = self.chain_body(&stages, tail, counter)?;

        Ok(self.close_chain(&chain.limit, counter, (body, tail.targets.skip)))
    }

    fn chain_header(&mut self, at: &'a Node, exit: u32) -> u32 {
        let header = self.node_with((Action::Nop, Flow::Goto(exit)), at, CHAIN_EVIDENCE);
        self.program.loop_headers.push(header);
        header
    }

    fn chain_body(
        &mut self,
        stages: &[Stage<'a>],
        tail: Tail<'a>,
        counter: Counter<'a>,
    ) -> Lowered {
        let increment = self.node(
            Action::Increment {
                variable: counter.index,
            },
            Flow::Goto(counter.header),
            tail.at,
        );
        let targets = Targets {
            skip: increment,
            stop: tail.targets.stop,
        };
        self.pipeline(stages, Tail { targets, ..tail })
    }

    // The synthetic counter is unobservable; starting at zero over-approximates `start..end`.
    fn close_chain(&mut self, limit: &Value, counter: Counter<'a>, branches: (u32, u32)) -> u32 {
        let guard = Condition::Less {
            variable: counter.index,
            limit: Value::Variable(counter.limit),
        };
        self.finish_loop(counter.header, guard, branches);
        let limit_init = Action::Assign {
            variable: counter.limit,
            value: limit.clone(),
        };
        let limit_init = self.node(limit_init, Flow::Goto(counter.header), counter.at);
        let index_init = Action::Assign {
            variable: counter.index,
            value: Value::Const(0),
        };

        self.node(index_init, Flow::Goto(limit_init), counter.at)
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    fn pipeline(&mut self, stages: &[Stage<'a>], tail: Tail<'a>) -> Lowered {
        let Some((stage, rest)) = stages.split_first() else {
            return self.step(consumer_step(tail), tail.at, tail.targets.skip);
        };
        if let Stage::FlatMap(closure) = stage {
            return self.flat_map(closure, rest, tail);
        }
        let next = self.pipeline(rest, tail)?;
        self.step(stage_step(*stage, tail.targets), tail.at, next)
    }

    fn step(&mut self, step: Step<'a>, at: &'a Node, next: u32) -> Lowered {
        let continued = match step.alternative {
            Some(alternative) => self.choice((next, alternative), at),
            None => next,
        };
        match step.callback {
            Some(callback) => self.invoke(callback, at, continued),
            None => Ok(continued),
        }
    }

    // The callback's tail must be a modeled adapter chain; downstream stages run per inner item.
    // A return anywhere in the callback, statements included, yields an unmodeled iterator.
    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    fn flat_map(&mut self, closure: &'a Node, rest: &[Stage<'a>], tail: Tail<'a>) -> Lowered {
        let body = self.closure_body(closure)?;
        let (statements, value) = match body.kind {
            NodeKind::Block if body.label.is_none() => self.statements(body)?,
            _ => (Vec::new(), Some(body)),
        };
        self.register_locals(&statements)?;
        let chain = match value {
            Some(value) => self.chain(value)?.filter(|chain| chain.standard),
            None => None,
        };
        let Some(chain) = chain else {
            let reason = "Flattened callback is not a modeled chain";
            return Ok(self.unsupported(tail.at, tail.targets.skip, reason));
        };
        let early = self.unsupported(
            tail.at,
            tail.targets.skip,
            "Flattened callback returns early",
        );
        self.returns.push(early);
        let result = (|| -> Lowered {
            let scan = self.scan_chain(&chain, rest, tail)?;
            let mut entry = self.chain_inputs(&chain, scan)?;
            for statement in statements.iter().rev() {
                entry = self.statement(statement, entry)?;
            }
            Ok(entry)
        })();
        self.returns.pop();

        result
    }

    pub(super) fn invoke(&mut self, callback: Callback<'a>, at: &'a Node, next: u32) -> Lowered {
        match callback {
            Callback::Closure(closure) => self.inline(closure, next),
            Callback::Function(id) => {
                // Item values are opaque; generic interface members stay unresolved.
                let definition = self.facts.tables.definition(id)?;
                let target = self.function_target(definition);
                let action = Action::Call {
                    callee: definition.name.clone(),
                    target: target.0,
                    arguments: Vec::new(),
                    may_modify: false,
                    operation: target.1,
                };
                Ok(self.node(action, Flow::Goto(next), at))
            }
        }
    }

    /// A function called as an item value, with its own generic parameters unresolved.
    fn function_target(
        &self,
        definition: &super::super::model::Definition,
    ) -> super::calls::Resolution {
        if self.facts.interface_member(definition)
            || (definition.standard && self.generic(definition))
        {
            return (Callee::Unresolved, None);
        }
        self.classified(definition, definition.operation.clone(), Some(false))
    }

    /// Whether type parameters are in scope of the function: its own, or its
    /// implementation's.
    fn generic(&self, definition: &super::super::model::Definition) -> bool {
        let Some(callable) = &definition.callable else {
            return false;
        };
        callable.generics > 0
            || callable
                .owner_type
                .as_deref()
                .and_then(|id| self.facts.type_of(id))
                .is_some_and(|owner| {
                    self.facts
                        .any_part(owner, 32, &|ty| ty.kind == TypeKind::Parameter)
                })
    }

    /// Inlining keeps capture identities, so captured places are the caller's places.
    pub(super) fn inline(&mut self, closure: &'a Node, next: u32) -> Lowered {
        let body = self.closure_body(closure)?;
        self.returns.push(next);
        let entry = self.expr(body, next);
        self.returns.pop();

        entry
    }

    fn closure_body(&self, closure: &'a Node) -> Result<&'a Node, String> {
        let parts = self.facts.body.children(closure)?;
        parts
            .last()
            .copied()
            .ok_or_else(|| format!("Closure {} has no body", closure.id))
    }
}

/// Arguments evaluated once before the first item: all but the inlined callback.
fn evaluated<'s, 'a>(
    arguments: &'s [&'a Node],
    callback: Option<u32>,
) -> impl Iterator<Item = &'a Node> + Clone + 's {
    let inlined = callback.map(|index| index as usize);
    arguments
        .iter()
        .copied()
        .enumerate()
        .filter(move |(i, _)| Some(*i) != inlined)
        .map(|(_, argument)| argument)
}

fn stage_step(stage: Stage<'_>, targets: Targets) -> Step<'_> {
    let (callback, alternative) = match stage {
        Stage::Pass => (None, None),
        Stage::Map(callback) => (Some(callback), None),
        Stage::Filter(callback) => (Some(callback), Some(targets.skip)),
        Stage::Skip => (None, Some(targets.skip)),
        Stage::Take => (None, Some(targets.stop)),
        Stage::TakeWhile(callback) => (Some(callback), Some(targets.stop)),
        Stage::FlatMap(_) => (None, None),
    };
    Step {
        callback,
        alternative,
    }
}

// Consumers run after the last stage; the item then advances or a search stops.
fn consumer_step(tail: Tail<'_>) -> Step<'_> {
    let (callback, alternative) = match tail.consumer {
        Consumer::Drain => (None, None),
        Consumer::Each(callback) => (Some(callback), None),
        Consumer::Search(callback) => (Some(callback), Some(tail.targets.stop)),
    };
    Step {
        callback,
        alternative,
    }
}
