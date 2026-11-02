//! Resolved standard iterator chains lower to synthetic scans with inlined callbacks.
//!
//! A chain is modeled only when its source is a stable slice/`Vec` traversal or a
//! primitive `Range`, and every stage is a compiler-resolved `Iterator` method on
//! that standard source. Each callback runs at most once per pulled source item;
//! filters, skips, and short-circuiting consumers become nondeterministic exits.
//!
//! Which methods are stages or consumers, and their per-item effects, are library
//! contract roles keyed like `rust:Iterator.map`; this module only lowers effects.
use super::*;
use contracts::{ConsumerEffect, Contracts, Iteration, StageEffect};
use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_span::sym;

/// Contract key of a resolved standard `Iterator` method. Upper-bound multiplicities
/// rely on its contract role, never on `size_hint`.
pub(super) fn iterator_method<'tcx>(ctx: &Context<'tcx>, expression: &Expr<'_>) -> Option<String> {
    let method = ctx.typeck.type_dependent_def_id(expression.hir_id)?;
    let iterator = ctx.tcx.get_diagnostic_item(sym::Iterator)?;
    if ctx.tcx.trait_of_assoc(method) != Some(iterator) || method.krate != iterator.krate {
        return None;
    }
    super::sysroot::trait_operation(ctx.tcx, iterator, method)
}

/// Arguments evaluated once before the first item: all but the inlined callback.
fn evaluated<'tcx>(
    arguments: &'tcx [Expr<'tcx>],
    callback: Option<u32>,
) -> impl DoubleEndedIterator<Item = &'tcx Expr<'tcx>> + Clone {
    let inlined = callback.map(|index| index as usize);
    let data = arguments.iter().enumerate();
    data.filter(move |(i, _)| Some(*i) != inlined)
        .map(|(_, argument)| argument)
}

#[derive(Clone, Copy)]
pub(super) enum Callback<'tcx> {
    Closure(&'tcx hir::Closure<'tcx>),
    Function(DefId),
}

#[derive(Clone, Copy)]
enum Stage<'tcx> {
    Pass,
    Map(Callback<'tcx>),
    Filter(Callback<'tcx>),
    Skip,
    Take,
    TakeWhile(Callback<'tcx>),
    FlatMap(&'tcx hir::Closure<'tcx>),
}

#[derive(Clone, Copy)]
enum Consumer<'tcx> {
    Drain,
    Each(Callback<'tcx>),
    Search(Callback<'tcx>),
}

struct Chain<'tcx> {
    root: &'tcx Expr<'tcx>,
    limit: Value,
    bits: u8,
    stages: Vec<Stage<'tcx>>,
    arguments: Vec<&'tcx Expr<'tcx>>,
    // Stages that dispatch to item trait impls are modeled only for standard items.
    standard: bool,
}

/// A flattening callback's statements and the adapter chain it returns.
struct Flattened<'tcx> {
    statements: &'tcx [hir::Stmt<'tcx>],
    chain: Option<Chain<'tcx>>,
}

#[derive(Clone, Copy)]
struct Tail<'tcx> {
    consumer: Consumer<'tcx>,
    targets: Targets,
    span: rustc_span::Span,
}

#[derive(Clone, Copy)]
struct Targets {
    skip: u32,
    stop: u32,
}

const MAX_STAGES: usize = 32;

#[derive(Clone, Copy)]
struct Counter {
    index: u32,
    limit: u32,
    header: u32,
    span: rustc_span::Span,
}

/// One item's effect: an optional callback, then an optional nondeterministic exit.
#[derive(Clone, Copy)]
struct Step<'tcx> {
    callback: Option<Callback<'tcx>>,
    alternative: Option<u32>,
}

impl<'tcx> Collector<'_, 'tcx> {
    /// `None` keeps ordinary call lowering, which rejects any unmodeled callback.
    pub(super) fn iterator_chain(
        &mut self,
        expression: &'tcx Expr<'tcx>,
        next: u32,
    ) -> Option<u32> {
        let ExprKind::MethodCall(_, receiver, arguments, _) = expression.kind else {
            return None;
        };
        let (consumer, contract) = self.consumer(expression, arguments)?;
        let mut chain = self.chain(receiver)?;
        if !chain.standard || !self.standard_dispatch(expression, receiver, &contract.dispatch) {
            let reason = "Iterator chain dispatches to a non-standard trait implementation";
            return Some(self.unsupported(expression.span, next, reason));
        }

        // Every argument but the inlined callback is evaluated, so other closures stay visible.
        chain
            .arguments
            .extend(evaluated(arguments, contract.callback));
        let targets = Targets {
            skip: next,
            stop: next,
        };

        let tail = Tail {
            consumer,
            targets,
            span: expression.span,
        };
        let scan = self.scan_chain(&chain, &[], tail);
        Some(self.chain_inputs(&chain, scan))
    }

    // Source places and stage arguments are evaluated once, before the first pull.
    fn chain_inputs(&mut self, chain: &Chain<'tcx>, scan: u32) -> u32 {
        let arguments = chain
            .arguments
            .iter()
            .rev()
            .fold(scan, |entry, argument| self.expr(argument, entry));
        match chain.root.kind {
            ExprKind::MethodCall(_, receiver, [], _) => self.expr(receiver, arguments),
            _ => self.expr(chain.root, arguments),
        }
    }

    /// Iterator calls whose callable arguments cannot all be resolved stay unknown.
    pub(super) fn unresolved_callback(&self, expression: &Expr<'tcx>) -> bool {
        let ExprKind::MethodCall(_, _, arguments, _) = expression.kind else {
            return false;
        };
        // Any callable component, such as a boxed `dyn Fn`, may be invoked per item.
        let callable = |argument: &Expr<'tcx>| {
            let ty = self.ctx.typeck.expr_ty_adjusted(argument);
            ty.walk()
                .any(|part| part.as_type().is_some_and(callable_kind))
        };
        iterator_method(self.ctx, expression).is_some() && arguments.iter().any(callable)
    }

    /// The consumer and its contract role.
    fn consumer(
        &self,
        expression: &Expr<'tcx>,
        arguments: &'tcx [Expr<'tcx>],
    ) -> Option<(Consumer<'tcx>, &'static Iteration<ConsumerEffect>)> {
        let key = iterator_method(self.ctx, expression)?;
        let contract = Contracts::embedded().consumer(&key)?;
        let callback = self.role_callback(contract, arguments)?;

        let consumer = match contract.effect {
            ConsumerEffect::Drain => Consumer::Drain,
            ConsumerEffect::Each => Consumer::Each(callback?),
            ConsumerEffect::Search => Consumer::Search(callback?),
        };
        Some((consumer, contract))
    }

    /// The role's resolved callback; `None` when the arity differs or the callback is
    /// unresolved, so ordinary call lowering keeps the call visible.
    fn role_callback<E>(
        &self,
        contract: &Iteration<E>,
        arguments: &'tcx [Expr<'tcx>],
    ) -> Option<Option<Callback<'tcx>>> {
        (arguments.len() == contract.arity as usize).then_some(())?;
        match contract.callback {
            Some(index) => self.callback(arguments.get(index as usize)?).map(Some),
            None => Some(None),
        }
    }

    fn chain(&self, mut receiver: &'tcx Expr<'tcx>) -> Option<Chain<'tcx>> {
        let mut links = Vec::new();
        while let ExprKind::MethodCall(_, inner, items, _) = receiver.kind
            && let Some(key) = iterator_method(self.ctx, receiver)
        {
            if links.len() >= MAX_STAGES || !self.ctx.typeck.expr_adjustments(inner).is_empty() {
                return None;
            }
            let (stage, contract) = self.stage(&key, items)?;
            let standard = self.standard_dispatch(receiver, inner, &contract.dispatch);
            links.push((stage, evaluated(items, contract.callback), standard));
            receiver = inner;
        }

        // Links were walked outermost first; the source's stage runs first.
        let (limit, bits) = self.source(receiver)?;
        Some(Chain {
            root: receiver,
            limit,
            bits,
            stages: links.iter().rev().map(|link| link.0).collect(),
            arguments: links.iter().rev().flat_map(|link| link.1.clone()).collect(),
            standard: links.iter().all(|link| link.2),
        })
    }

    fn stage(
        &self,
        key: &str,
        items: &'tcx [Expr<'tcx>],
    ) -> Option<(Stage<'tcx>, &'static Iteration<StageEffect>)> {
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
    fn source(&self, root: &'tcx Expr<'tcx>) -> Option<(Value, u8)> {
        if matches!(root.kind, ExprKind::Struct(..)) {
            return self.range_source(root);
        }
        let ExprKind::MethodCall(_, receiver, [], _) = root.kind else {
            return None;
        };

        if !extent::has_role(self.ctx, root, contracts::ExtentRole::Traversal) {
            return None;
        }
        let place = extent::place(self.ctx, receiver)?;
        let label = self.extents.get(&place.key())?.clone();
        let bits = self.ctx.tcx.data_layout.pointer_size().bits() as u8;
        Some((Value::Input(label), bits))
    }

    // A primitive `start..end` yields at most `end` items, whatever its start.
    fn range_source(&self, root: &Expr<'tcx>) -> Option<(Value, u8)> {
        let (_, end, bits) = self.range_parts(root)?;
        let limit = self.value(end);
        (!matches!(limit, Value::Unknown)).then_some((limit, bits))
    }

    pub(super) fn callback(&self, argument: &Expr<'tcx>) -> Option<Callback<'tcx>> {
        match argument.kind {
            ExprKind::Closure(closure) if matches!(closure.kind, hir::ClosureKind::Closure) => {
                Some(Callback::Closure(closure))
            }
            ExprKind::Path(ref path) => match self.ctx.typeck.qpath_res(path, argument.hir_id) {
                Res::Local(id) => self.closures.get(&id).copied().map(Callback::Closure),
                Res::Def(DefKind::Fn | DefKind::AssocFn, id) => Some(Callback::Function(id)),
                _ => None,
            },
            _ => None,
        }
    }

    /// Scans `chain`, then `rest` per item; `tail.targets` are the scan exit and chain stop.
    fn scan_chain(&mut self, chain: &Chain<'tcx>, rest: &[Stage<'tcx>], tail: Tail<'tcx>) -> u32 {
        let counter = self.chain_counter(chain.bits, tail.span, tail.targets.skip);
        let stages: Vec<_> = chain.stages.iter().chain(rest).copied().collect();
        let body = self.chain_body(&stages, tail, counter);
        self.close_chain(&chain.limit, counter, (body, tail.targets.skip))
    }

    fn chain_counter(&mut self, bits: u8, span: rustc_span::Span, exit: u32) -> Counter {
        let index = self.variable("chain-index".into(), Value::Unknown, bits);
        let limit = self.variable("chain-limit".into(), Value::Unknown, bits);
        let header = self.chain_header(span, exit);
        Counter {
            index,
            limit,
            header,
            span,
        }
    }

    fn chain_header(&mut self, span: rustc_span::Span, exit: u32) -> u32 {
        let header = self.loop_header(span, exit);
        if let Some(source) = self.program.nodes[header as usize].source.as_mut() {
            source.evidence = CHAIN_EVIDENCE.into();
        }
        header
    }

    fn chain_body(&mut self, stages: &[Stage<'tcx>], tail: Tail<'tcx>, counter: Counter) -> u32 {
        let increment = self.node(
            Action::Increment {
                variable: counter.index,
            },
            Flow::Goto(counter.header),
            tail.span,
        );
        let targets = Targets {
            skip: increment,
            stop: tail.targets.stop,
        };
        self.pipeline(stages, Tail { targets, ..tail })
    }

    // The synthetic counter is unobservable; starting at zero over-approximates `start..end`.
    fn close_chain(&mut self, limit: &Value, counter: Counter, branches: (u32, u32)) -> u32 {
        let guard = Condition::Less {
            variable: counter.index,
            limit: Value::Variable(counter.limit),
        };
        self.finish_loop(counter.header, guard, branches);
        let span = counter.span;

        let limit_init = Action::Assign {
            variable: counter.limit,
            value: limit.clone(),
        };
        let limit_init = self.node(limit_init, Flow::Goto(counter.header), span);
        let index_init = Action::Assign {
            variable: counter.index,
            value: Value::Const(0),
        };
        self.node(index_init, Flow::Goto(limit_init), span)
    }

    fn pipeline(&mut self, stages: &[Stage<'tcx>], tail: Tail<'tcx>) -> u32 {
        let Some((stage, rest)) = stages.split_first() else {
            return self.step(consumer_step(tail), tail.span, tail.targets.skip);
        };
        if let Stage::FlatMap(closure) = stage {
            return self.flat_map(closure, rest, tail);
        }
        self.stage_entry(*stage, rest, tail)
    }

    fn stage_entry(&mut self, stage: Stage<'tcx>, rest: &[Stage<'tcx>], tail: Tail<'tcx>) -> u32 {
        let next = self.pipeline(rest, tail);
        self.step(stage_step(stage, tail.targets), tail.span, next)
    }

    fn step(&mut self, step: Step<'tcx>, span: rustc_span::Span, next: u32) -> u32 {
        let continued = match step.alternative {
            Some(alternative) => self.choice((next, alternative), span),
            None => next,
        };
        match step.callback {
            Some(callback) => self.invoke(callback, span, continued),
            None => continued,
        }
    }

    // The callback's tail must be a modeled adapter chain; downstream stages run per inner item.
    fn flat_map(
        &mut self,
        closure: &'tcx hir::Closure<'tcx>,
        rest: &[Stage<'tcx>],
        tail: Tail<'tcx>,
    ) -> u32 {
        let flattened = self.flattened(closure);
        self.nested_scan(flattened, rest, tail)
    }

    fn flattened(&mut self, closure: &'tcx hir::Closure<'tcx>) -> Flattened<'tcx> {
        let body = self.ctx.tcx.hir_body(closure.body);
        let (statements, value) = match body.value.kind {
            ExprKind::Block(block, None) => (block.stmts, block.expr),
            _ => (&[][..], Some(body.value)),
        };

        self.register_locals(statements);
        let chain = value
            .and_then(|value| self.chain(value))
            .filter(|c| c.standard);
        Flattened { statements, chain }
    }

    // Exhausting the inner scan pulls the next outer item; stops still end the whole chain.
    // A return anywhere in the callback, statements included, yields an unmodeled iterator.
    fn nested_scan(
        &mut self,
        flattened: Flattened<'tcx>,
        rest: &[Stage<'tcx>],
        tail: Tail<'tcx>,
    ) -> u32 {
        let Some(chain) = flattened.chain else {
            let reason = "Flattened callback is not a modeled chain";
            return self.unsupported(tail.span, tail.targets.skip, reason);
        };
        let reason = "Flattened callback returns early";
        let early = self.unsupported(tail.span, tail.targets.skip, reason);
        self.returning(early, |this| {
            let scan = this.scan_chain(&chain, rest, tail);
            let inputs = this.chain_inputs(&chain, scan);
            this.sequence(flattened.statements, inputs, Self::statement)
        })
    }

    pub(super) fn invoke(
        &mut self,
        callback: Callback<'tcx>,
        span: rustc_span::Span,
        next: u32,
    ) -> u32 {
        match callback {
            Callback::Closure(closure) => self.inline(closure, next),
            Callback::Function(id) => {
                // Item values are opaque; generic trait items stay unresolved.
                let identity = ty::GenericArgs::identity_for_item(self.ctx.tcx, id);
                let (target, operation) = self.target(id, identity);
                let action = Action::Call {
                    callee: self.ctx.tcx.def_path_str(id),
                    target,
                    arguments: Vec::new(),
                    may_modify: false,
                    operation,
                };
                self.node(action, Flow::Goto(next), span)
            }
        }
    }

    /// Inlining keeps HIR capture identities, so captured places are the caller's places.
    pub(super) fn inline(&mut self, closure: &'tcx hir::Closure<'tcx>, next: u32) -> u32 {
        let body = self.ctx.tcx.hir_body(closure.body);
        self.returning(next, |this| this.expr(body.value, next))
    }

    // Returns inside an inlined body resume at `target` instead of leaving the owner.
    fn returning<T>(&mut self, target: u32, lower: impl FnOnce(&mut Self) -> T) -> T {
        self.returns.push(target);
        let result = lower(self);
        self.returns.pop();
        result
    }
}

fn stage_step<'tcx>(stage: Stage<'tcx>, targets: Targets) -> Step<'tcx> {
    let (callback, alternative) = match stage {
        Stage::Pass => (None, None),
        Stage::Map(callback) => (Some(callback), None),
        Stage::Filter(callback) => (Some(callback), Some(targets.skip)),
        Stage::Skip => (None, Some(targets.skip)),
        Stage::Take => (None, Some(targets.stop)),
        Stage::TakeWhile(callback) => (Some(callback), Some(targets.stop)),
        Stage::FlatMap(_) => unreachable!("flattening is lowered as a nested scan"),
    };
    Step {
        callback,
        alternative,
    }
}

// Consumers run after the last stage; the item then advances or a search stops.
fn consumer_step<'tcx>(tail: Tail<'tcx>) -> Step<'tcx> {
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

const CHAIN_EVIDENCE: &str = "live typed HIR; resolved standard iterator chain over a stable extent or primitive range; callbacks inlined at each invocation";

fn callable_kind(ty: Ty<'_>) -> bool {
    matches!(
        ty.kind(),
        ty::Closure(..)
            | ty::FnDef(..)
            | ty::FnPtr(..)
            | ty::Param(_)
            | ty::Dynamic(..)
            | ty::Alias(..)
    )
}
