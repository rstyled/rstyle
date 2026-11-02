//! Resolved standard iterator chains lower to synthetic scans with inlined callbacks.
//!
//! A chain is modeled only when its source is a stable slice/`Vec` traversal or a
//! primitive `Range`, and every stage is a compiler-resolved `Iterator` method on
//! that standard source. Each callback runs at most once per pulled source item;
//! filters, skips, and short-circuiting consumers become nondeterministic exits.
use super::*;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_span::sym;

/// Upper-bound multiplicities rely on standard contracts, never on `size_hint`.
pub(super) fn iterator_method<'tcx>(ctx: &Context<'tcx>, expression: &Expr<'_>) -> Option<String> {
    let method = ctx.typeck.type_dependent_def_id(expression.hir_id)?;
    let iterator = ctx.tcx.get_diagnostic_item(sym::Iterator)?;
    (ctx.tcx.trait_of_assoc(method) == Some(iterator) && method.krate == iterator.krate)
        .then(|| ctx.tcx.item_name(method).to_string())
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
        let consumer = self.consumer(expression, arguments)?;
        let mut chain = self.chain(receiver)?;

        let data = arguments.iter().filter(|a| self.callback(a).is_none());
        chain.arguments.extend(data);
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
        let callable = |argument: &Expr<'tcx>| {
            matches!(
                self.ctx
                    .typeck
                    .expr_ty_adjusted(argument)
                    .peel_refs()
                    .kind(),
                ty::Closure(..)
                    | ty::FnDef(..)
                    | ty::FnPtr(..)
                    | ty::Param(_)
                    | ty::Dynamic(..)
                    | ty::Alias(..)
            )
        };
        iterator_method(self.ctx, expression).is_some() && arguments.iter().any(callable)
    }

    fn consumer(
        &self,
        expression: &Expr<'tcx>,
        arguments: &'tcx [Expr<'tcx>],
    ) -> Option<Consumer<'tcx>> {
        let name = iterator_method(self.ctx, expression)?;
        let callback = |index: usize| arguments.get(index).and_then(|a| self.callback(a));
        Some(match (name.as_str(), arguments.len()) {
            ("sum" | "product" | "count" | "last" | "min" | "max" | "collect", 0) => {
                Consumer::Drain
            }
            ("min_by_key" | "max_by_key" | "min_by" | "max_by" | "for_each", 1) => {
                Consumer::Each(callback(0)?)
            }
            ("fold", 2) => Consumer::Each(callback(1)?),
            ("any" | "all" | "find" | "position" | "find_map", 1) => Consumer::Search(callback(0)?),
            _ => return None,
        })
    }

    fn chain(&self, mut receiver: &'tcx Expr<'tcx>) -> Option<Chain<'tcx>> {
        let mut stages = Vec::new();
        let mut arguments = Vec::new();
        while let ExprKind::MethodCall(_, inner, items, _) = receiver.kind
            && let Some(name) = iterator_method(self.ctx, receiver)
        {
            if stages.len() >= MAX_STAGES || !self.ctx.typeck.expr_adjustments(inner).is_empty() {
                return None;
            }

            let stage = self.stage(&name, items)?;
            if matches!(stage, Stage::Skip | Stage::Take) {
                arguments.extend(items.iter().rev());
            }
            stages.push(stage);
            receiver = inner;
        }

        stages.reverse();
        arguments.reverse();
        let (limit, bits) = self.source(receiver)?;
        Some(Chain {
            root: receiver,
            limit,
            bits,
            stages,
            arguments,
        })
    }

    fn stage(&self, name: &str, items: &'tcx [Expr<'tcx>]) -> Option<Stage<'tcx>> {
        let callback = || items.first().and_then(|a| self.callback(a));
        Some(match (name, items.len()) {
            ("copied" | "cloned" | "enumerate" | "rev", 0) => Stage::Pass,
            ("map" | "inspect", 1) => Stage::Map(callback()?),
            ("filter" | "filter_map" | "skip_while", 1) => Stage::Filter(callback()?),
            ("skip", 1) => Stage::Skip,
            ("take", 1) => Stage::Take,
            ("take_while", 1) => Stage::TakeWhile(callback()?),
            ("flat_map", 1) => match callback()? {
                Callback::Closure(closure) => Stage::FlatMap(closure),
                Callback::Function(_) => return None,
            },
            _ => return None,
        })
    }

    // Slice and `Vec` traversals yield at most their stable length.
    fn source(&self, root: &'tcx Expr<'tcx>) -> Option<(Value, u8)> {
        if let ExprKind::Struct(_, fields, hir::StructTailExpr::None) = root.kind {
            return self.range_source(root, fields);
        }
        let ExprKind::MethodCall(_, receiver, [], _) = root.kind else {
            return None;
        };

        if !extent::standard_method(self.ctx, root, "iter") {
            return None;
        }
        let place = extent::place(self.ctx, receiver)?;
        let label = self.extents.get(&place.key())?.clone();
        let bits = self.ctx.tcx.data_layout.pointer_size().bits() as u8;
        Some((Value::Input(label), bits))
    }

    // A primitive `start..end` yields at most `end` items, whatever its start.
    fn range_source(
        &self,
        root: &Expr<'tcx>,
        fields: &[hir::ExprField<'tcx>],
    ) -> Option<(Value, u8)> {
        let [start, end] = fields else {
            return None;
        };
        let ty::Adt(definition, arguments) = self.ctx.typeck.expr_ty(root).kind() else {
            return None;
        };
        let bits = self.bits(arguments.type_at(0))?;

        let limit = self.value(end.expr);
        (Some(definition.did()) == self.ctx.tcx.lang_items().range_struct()
            && start.ident.name.as_str() == "start"
            && end.ident.name.as_str() == "end"
            && !matches!(limit, Value::Unknown))
        .then_some((limit, bits))
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
        let (statements, chain) = self.flattened(closure);
        let scan = self.nested_scan(chain.as_ref(), rest, tail);
        self.sequence(statements, scan, Self::statement)
    }

    fn flattened(
        &mut self,
        closure: &'tcx hir::Closure<'tcx>,
    ) -> (&'tcx [hir::Stmt<'tcx>], Option<Chain<'tcx>>) {
        let body = self.ctx.tcx.hir_body(closure.body);
        let (statements, value) = match body.value.kind {
            ExprKind::Block(block, None) => (block.stmts, block.expr),
            _ => (&[][..], Some(body.value)),
        };
        self.register_locals(statements);
        (statements, value.and_then(|value| self.chain(value)))
    }

    // Exhausting the inner scan pulls the next outer item; stops still end the whole chain.
    fn nested_scan(
        &mut self,
        chain: Option<&Chain<'tcx>>,
        rest: &[Stage<'tcx>],
        tail: Tail<'tcx>,
    ) -> u32 {
        let Some(chain) = chain else {
            let reason = "Flattened callback is not a modeled chain";
            return self.unsupported(tail.span, tail.targets.skip, reason);
        };
        let reason = "Flattened callback returns early";
        let early = self.unsupported(tail.span, tail.targets.skip, reason);
        self.returning(early, |this| {
            let scan = this.scan_chain(chain, rest, tail);
            this.chain_inputs(chain, scan)
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
            Callback::Function(id) => self.node(
                Action::Call {
                    callee: self.ctx.tcx.def_path_str(id),
                    may_modify: false,
                },
                Flow::Goto(next),
                span,
            ),
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
