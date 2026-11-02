//! Bounded, typed HIR lowering for advisory runtime bounds. Never uses clone admission.
mod branches;
mod calls;
mod chains;
mod control;
mod dispatch;
mod extent;
mod frame;
mod iteration;
mod primitive;
mod sysroot;
mod values;
use super::hir::Context;
use rstyle_core::{
    advisory::runtime_bounds::{CAPTURE_ENV, Input, KEY, MAX_NODES, MAX_VARIABLES, Observation},
    runtime_bounds::*,
};
use rstyle_pdg::HirFnFacts;
use rustc_hir::{
    self as hir, Expr, ExprKind, HirId, PatKind, StmtKind, def::Res, def_id::LocalDefId,
};
use rustc_middle::ty::{self, Ty};
use std::collections::HashMap;

pub(super) fn capture<B, S>(
    ctx: &Context<'_>,
    id: LocalDefId,
    facts: &mut HirFnFacts<B, S>,
) -> Result<(), String> {
    if std::env::var_os(CAPTURE_ENV).as_deref() != Some(std::ffi::OsStr::new("1")) {
        return Ok(());
    }

    let input = match ctx.tcx.hir_node_by_def_id(id).body_id() {
        Some(body) => Collector::collect(ctx, ctx.tcx.hir_body(body)),
        None => Input::Unavailable {
            reason: "Compiler owner has no HIR body".into(),
        },
    };
    let observation = Observation {
        schema: KEY.into(),
        owner: crate::ids::def_id_str(ctx.tcx, id.to_def_id()),
        krate: calls::crate_identity(ctx.tcx, rustc_hir::def_id::LOCAL_CRATE),
        input,
    };

    facts.build.frontend_options.insert(
        KEY.into(),
        serde_json::to_value(observation).map_err(|e| e.to_string())?,
    );
    Ok(())
}

#[derive(Clone, Copy)]
struct LoopSite {
    owner: HirId,
    span: rustc_span::Span,
    next: u32,
}

struct Collector<'a, 'tcx> {
    ctx: &'a Context<'tcx>,
    program: Program,
    locals: HashMap<HirId, u32>,
    loops: HashMap<HirId, (u32, u32)>,
    exhausted: bool,
    depth: usize,
    work: usize,
    extents: HashMap<String, String>,
    closures: HashMap<HirId, &'tcx hir::Closure<'tcx>>,
    returns: Vec<u32>,
    formals: HashMap<HirId, u32>,
}

impl<'a, 'tcx> Collector<'a, 'tcx> {
    fn collect(ctx: &'a Context<'tcx>, body: &'tcx hir::Body<'tcx>) -> Input {
        let mut collector = Self::new(ctx);

        if body.params.len() > MAX_VARIABLES {
            return Input::BudgetExhausted {
                reason: "HIR parameter capture budget exhausted".into(),
            };
        }

        let frame = frame::inspect(ctx, body);
        if frame.exhausted {
            return Input::BudgetExhausted {
                reason: "HIR frame inspection budget exhausted".into(),
            };
        }
        collector.prepare(body, frame);

        if collector.exhausted {
            Input::BudgetExhausted {
                reason: "HIR CFG capture node, variable, or nesting budget exhausted".into(),
            }
        } else {
            Input::Captured {
                program: collector.program,
            }
        }
    }

    fn prepare(&mut self, body: &'tcx hir::Body<'tcx>, frame: frame::Frame) {
        self.install_extents(body.params, frame.extents);
        self.parameters(body.params);
        self.framed_body(body.value, frame.reason);
    }

    fn framed_body(&mut self, value: &'tcx Expr<'tcx>, reason: Option<String>) {
        match reason {
            Some(reason) => self.reject_body(value.span, &reason),
            None => self.capture_body(value),
        }
    }

    fn reject_body(&mut self, span: rustc_span::Span, reason: &str) {
        let end = self.node(Action::Nop, Flow::Return, span);
        let entry = self.unsupported(span, end, reason);
        self.program.entry = entry;
    }

    // Formal positions first: extent provenance records formal-rooted places.
    fn install_extents(&mut self, parameters: &[hir::Param<'tcx>], places: Vec<extent::Place>) {
        self.formal_positions(parameters);
        for place in places {
            self.install_extent(place);
        }
    }

    fn install_extent(&mut self, place: extent::Place) {
        let bits = self.ctx.tcx.data_layout.pointer_size().bits() as u8;
        self.variable(place.label.clone(), Value::Input(place.label.clone()), bits);
        self.program.inputs.push(place.label.clone());
        self.record_extent(place);
    }

    fn record_extent(&mut self, place: extent::Place) {
        self.program.extents.push(self.provenance(&place));
        self.extents.insert(place.key(), place.label);
    }

    fn capture_body(&mut self, value: &'tcx Expr<'tcx>) {
        let end = self.node(Action::Nop, Flow::Return, value.span);
        let entry = self.expr(value, end);
        self.program.entry = entry;
    }

    fn new(ctx: &'a Context<'tcx>) -> Self {
        Self {
            ctx,
            program: Program {
                inputs: Vec::new(),
                variables: Vec::new(),
                nodes: Vec::new(),
                entry: 0,
                loop_headers: Vec::new(),
                formals: Vec::new(),
                extents: Vec::new(),
                arithmetic: if ctx.tcx.sess.overflow_checks() {
                    Arithmetic::Checked
                } else {
                    Arithmetic::Wrapping
                },
            },
            locals: HashMap::new(),
            loops: HashMap::new(),
            exhausted: false,
            depth: 0,
            work: 0,
            extents: HashMap::new(),
            closures: HashMap::new(),
            returns: Vec::new(),
            formals: HashMap::new(),
        }
    }

    fn variable(&mut self, name: String, initial: Value, bits: u8) -> u32 {
        if self.program.variables.len() >= MAX_VARIABLES {
            self.exhausted = true;
            return 0;
        }
        let index = self.program.variables.len() as u32;
        self.program.variables.push(Variable {
            name,
            initial,
            bits,
        });
        index
    }

    fn node(&mut self, action: Action, flow: Flow, span: rustc_span::Span) -> u32 {
        if self.program.nodes.len() >= MAX_NODES {
            self.exhausted = true;
            return 0;
        }

        let at = self
            .ctx
            .tcx
            .sess
            .source_map()
            .lookup_char_pos(span.source_callsite().lo());
        let source = Some(Source {
            file: at.file.name.prefer_local_unconditionally().to_string(),
            line: at.line,
            column: at.col.0 + 1,
            evidence:
                "live typed HIR; primitive unsigned semantics; no iterator trait size contract"
                    .into(),
        });

        let index = self.program.nodes.len() as u32;
        self.program.nodes.push(Node {
            action,
            flow,
            source,
        });
        index
    }

    fn unsupported(&mut self, span: rustc_span::Span, next: u32, reason: &str) -> u32 {
        self.node(
            Action::Unsupported {
                reason: reason.into(),
            },
            Flow::Goto(next),
            span,
        )
    }

    fn block(&mut self, block: &'tcx hir::Block<'tcx>, next: u32) -> u32 {
        if !self.charge(block.stmts.len()) {
            return next;
        }
        self.register_locals(block.stmts);
        self.block_entries(block, next)
    }

    // Register before reverse lowering, retaining lexical assignment order in the CFG.
    fn register_locals(&mut self, statements: &[hir::Stmt<'tcx>]) {
        for statement in statements {
            if let StmtKind::Let(local) = statement.kind {
                match local_closure(local) {
                    Some((id, closure)) => _ = self.closures.insert(id, closure),
                    None => self.register_local(local),
                }
            }
        }
    }

    fn register_local(&mut self, local: &hir::LetStmt<'tcx>) {
        if let PatKind::Binding(hir::BindingMode(hir::ByRef::No, _), id, name, None) =
            local.pat.kind
            && let Some(bits) = self.bits(self.ctx.typeck.pat_ty(local.pat))
        {
            let variable = self.variable(name.name.to_string(), Value::Unknown, bits);
            self.locals.insert(id, variable);
        }
    }

    fn block_entries(&mut self, block: &'tcx hir::Block<'tcx>, next: u32) -> u32 {
        let mut entry = block.expr.map_or(next, |value| self.expr(value, next));
        for statement in block.stmts.iter().rev() {
            entry = self.statement(statement, entry);
        }
        entry
    }

    fn statement(&mut self, statement: &hir::Stmt<'tcx>, next: u32) -> u32 {
        match statement.kind {
            StmtKind::Expr(value) | StmtKind::Semi(value) => self.expr(value, next),
            StmtKind::Let(local) => self.local(local, next),
            StmtKind::Item(_) => next,
        }
    }

    fn local(&mut self, local: &'tcx hir::LetStmt<'tcx>, next: u32) -> u32 {
        // Immutable local closures run only where they are directly invoked.
        if let PatKind::Binding(_, id, _, None) = local.pat.kind
            && self.closures.contains_key(&id)
        {
            return next;
        }
        match self.local_action(local) {
            Ok(Some(action)) => self.local_assignment(local, action, next),
            Ok(None) => local.init.map_or(next, |value| self.expr(value, next)),
            Err(reason) => self.unsupported(local.span, next, reason),
        }
    }

    fn local_assignment(
        &mut self,
        local: &'tcx hir::LetStmt<'tcx>,
        action: Action,
        next: u32,
    ) -> u32 {
        let assignment = self.node(action, Flow::Goto(next), local.span);
        local
            .init
            .map_or(assignment, |value| self.expr(value, assignment))
    }

    fn local_action(&self, local: &hir::LetStmt<'tcx>) -> Result<Option<Action>, &'static str> {
        if local.els.is_some() {
            return Err("let-else control flow is unmodeled");
        }
        if let PatKind::Binding(_, id, _, None) = local.pat.kind
            && let Some(variable) = self.locals.get(&id).copied()
        {
            let value = local.init.map_or(Value::Unknown, |value| self.value(value));
            return Ok(Some(Action::Assign { variable, value }));
        }

        Ok(None)
    }

    fn expr(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        if !self.enter_expression() {
            return next;
        }

        let result = self.lower(expression, next);
        self.depth -= 1;
        result
    }

    fn enter_expression(&mut self) -> bool {
        if !self.charge(1) {
            return false;
        }
        if self.depth >= 256 {
            self.exhausted = true;
            return false;
        }
        self.depth += 1;
        true
    }

    fn charge(&mut self, work: usize) -> bool {
        self.work = self.work.saturating_add(work);
        if self.exhausted || self.work > MAX_NODES * 4 {
            self.exhausted = true;
            return false;
        }
        true
    }
}

fn local_closure<'tcx>(local: &hir::LetStmt<'tcx>) -> Option<(HirId, &'tcx hir::Closure<'tcx>)> {
    let PatKind::Binding(hir::BindingMode(hir::ByRef::No, hir::Mutability::Not), id, _, None) =
        local.pat.kind
    else {
        return None;
    };
    match local.init?.kind {
        ExprKind::Closure(closure)
            if local.els.is_none() && matches!(closure.kind, hir::ClosureKind::Closure) =>
        {
            Some((id, closure))
        }
        _ => None,
    }
}
