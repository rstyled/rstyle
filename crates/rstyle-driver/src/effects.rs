//! One HIR walk per function: statement count, resolved calls, and effects
//! that need no config (`unsafe`, `static mut`, indirect calls).

use crate::args::{self, ParamIndex};
use crate::extract::line_of;
use crate::ids;
use rstyle_core::facts::{ArgSource, CallEvent, Edge, EffectEvent, EffectKind};
use rstyle_core::paths::strip_generics;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{BlockCheckMode, Body, Expr, ExprKind, PatKind, QPath, Stmt, UnsafeSource};
use rustc_middle::hir::nested_filter::OnlyBodies;
use rustc_middle::ty::{self, TyCtxt, TypeckResults};
use rustc_span::Span;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct BodyFacts {
    pub statements: usize,
    pub calls: Vec<CallEvent>,
    pub effects: Vec<EffectEvent>,
    pub edges: Vec<Edge>,
    /// Mentions of each parameter (`self` excluded) in the body.
    pub param_uses: Vec<usize>,
}

/// Index of each simple-binding parameter, `self` excluded.
fn param_index(tcx: TyCtxt<'_>, def_id: LocalDefId, body: &Body<'_>) -> ParamIndex {
    let skip = tcx.hir_node_by_def_id(def_id).fn_sig().map_or(0, |sig| {
        usize::from(sig.decl.implicit_self().has_implicit_self())
    });
    body.params
        .iter()
        .skip(skip)
        .enumerate()
        .filter(|(_, param)| matches!(param.pat.kind, PatKind::Binding(..)))
        .map(|(index, param)| (param.pat.hir_id, index))
        .collect()
}

pub fn collect<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: LocalDefId,
    body: &'tcx Body<'tcx>,
    root: &Path,
) -> BodyFacts {
    let params = param_index(tcx, def_id, body);
    let out = BodyFacts {
        param_uses: vec![0; body.params.len()],
        ..BodyFacts::default()
    };
    let mut walker = Walker {
        tcx,
        typeck: tcx.typeck(def_id),
        root: root.to_path_buf(),
        params,
        out,
        generated_depth: 0,
        unsafe_depth: 0,
    };

    walker.visit_body(body);
    walker.out
}

/// A resolved call target and what was passed to it.
struct Target {
    def_id: DefId,
    dynamic: bool,
    args: Vec<ArgSource>,
}

struct Walker<'tcx> {
    tcx: TyCtxt<'tcx>,
    typeck: &'tcx TypeckResults<'tcx>,
    root: PathBuf,
    params: ParamIndex,
    out: BodyFacts,
    /// Nonzero while inside macro/desugaring-generated code. Generated statements
    /// are counted once (the outermost), user-written ones always.
    generated_depth: usize,
    /// Nonzero while inside a user-written `unsafe` block.
    unsafe_depth: usize,
}

impl<'tcx> Walker<'tcx> {
    fn line(&self, span: Span) -> usize {
        line_of(self.tcx, span)
    }

    fn effect(&mut self, kind: EffectKind, detail: String, span: Span) {
        let line = self.line(span);
        self.out.effects.push(EffectEvent { kind, detail, line });
    }

    fn counts(&self, span: Span) -> bool {
        !span.from_expansion() || self.generated_depth == 0
    }

    fn call(&mut self, target: Target, expr: &Expr<'tcx>) {
        let Target {
            def_id, dynamic, ..
        } = target;

        if !is_fn(self.tcx, def_id) {
            return;
        }
        self.edge(target, expr);
        if def_id.is_local() {
            return;
        }
        let callee = strip_generics(&self.tcx.def_path_str(def_id));
        let line = self.line(expr.span);
        if self.unsafe_depth == 0 && !expr.span.from_expansion() && is_unsafe_fn(self.tcx, def_id) {
            self.effect(
                EffectKind::Unsafe,
                format!("call to unsafe fn {callee}"),
                expr.span,
            );
        }
        self.out.calls.push(CallEvent {
            callee,
            line,
            dynamic,
        });
    }

    /// Records a call into workspace code (same crate included) for the call graph.
    fn edge(&mut self, target: Target, expr: &Expr<'tcx>) {
        let Target {
            def_id,
            dynamic,
            args,
        } = target;

        if !ids::in_workspace(self.tcx, def_id, &self.root) {
            return;
        }
        let line = self.line(expr.span);
        let callee = ids::def_id_str(self.tcx, def_id);
        self.out.edges.push(Edge {
            callee,
            line,
            dynamic,
            args,
        });
    }

    fn indirect(&mut self, what: &str, expr: &Expr<'tcx>) {
        let line = self.line(expr.span);
        self.out.calls.push(CallEvent {
            callee: what.into(),
            line,
            dynamic: true,
        });
    }

    fn sources(&self, args: &[Expr<'tcx>]) -> Vec<ArgSource> {
        args.iter()
            .map(|arg| args::classify(self.typeck, &self.params, arg))
            .collect()
    }

    fn on_call(&mut self, expr: &Expr<'tcx>, callee: &Expr<'tcx>, args: &[Expr<'tcx>]) {
        match self.typeck.expr_ty(callee).kind() {
            ty::FnDef(def_id, _) => {
                // `Type::method(recv, ..)`: the receiver is not a parameter in `FnFacts`.
                let skip = usize::from(has_self(self.tcx, *def_id));
                let sources = self.sources(args.get(skip..).unwrap_or_default());
                self.call(target(*def_id, false, sources), expr);
            }
            ty::Closure(..) | ty::CoroutineClosure(..) => {}
            ty::FnPtr(..) => self.indirect("<fn pointer>", expr),
            _ => self.indirect("<indirect callable>", expr),
        }
    }

    fn on_method(&mut self, expr: &Expr<'tcx>, receiver: &Expr<'tcx>, args: &[Expr<'tcx>]) {
        let Some(def_id) = self.typeck.type_dependent_def_id(expr.hir_id) else {
            return;
        };
        let receiver_ty = self.typeck.expr_ty_adjusted(receiver).peel_refs();
        let dynamic = matches!(receiver_ty.kind(), ty::Dynamic(..));
        let sources = self.sources(args);

        self.call(target(def_id, dynamic, sources), expr);
    }

    fn on_path(&mut self, expr: &Expr<'tcx>, qpath: &QPath<'tcx>) {
        if let Res::Local(id) = self.typeck.qpath_res(qpath, expr.hir_id)
            && let Some(&index) = self.params.get(&id)
        {
            self.out.param_uses[index] += 1;
        }
        if let Res::Def(DefKind::Static { mutability, .. }, def_id) =
            self.typeck.qpath_res(qpath, expr.hir_id)
            && mutability.is_mut()
        {
            let name = strip_generics(&self.tcx.def_path_str(def_id));
            self.effect(EffectKind::Global, format!("static mut {name}"), expr.span);
        }
    }
}

fn target(def_id: DefId, dynamic: bool, args: Vec<ArgSource>) -> Target {
    Target {
        def_id,
        dynamic,
        args,
    }
}

fn has_self(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.opt_associated_item(def_id)
        .is_some_and(|item| item.is_method())
}

fn is_fn(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn)
}

fn is_unsafe_fn(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.fn_sig(def_id)
        .skip_binder()
        .skip_binder()
        .safety()
        .is_unsafe()
}

impl<'tcx> Visitor<'tcx> for Walker<'tcx> {
    type NestedFilter = OnlyBodies;

    fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
        self.tcx
    }

    // rstyle-allow(mutation_per_target, side_effect_density): visitor accumulates results in self by design
    fn visit_stmt(&mut self, stmt: &'tcx Stmt<'tcx>) {
        self.out.statements += usize::from(self.counts(stmt.span));
        let generated = stmt.span.from_expansion();
        self.generated_depth += usize::from(generated);
        intravisit::walk_stmt(self, stmt);

        self.generated_depth -= usize::from(generated);
    }

    // rstyle-allow(mutation_per_target, side_effect_density): visitor accumulates results in self by design
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        let generated = expr.span.from_expansion();

        self.generated_depth += usize::from(generated);
        match expr.kind {
            ExprKind::Call(callee, args) => self.on_call(expr, callee, args),
            ExprKind::MethodCall(_, receiver, args, _) => self.on_method(expr, receiver, args),
            ExprKind::Path(ref qpath) => self.on_path(expr, qpath),
            _ => {}
        }
        self.walk_expr_kind(expr);
        self.generated_depth -= usize::from(generated);
    }

    // rstyle-allow(side_effect_density): visitor accumulates results in self by design
    fn visit_block(&mut self, block: &'tcx rustc_hir::Block<'tcx>) {
        if let Some(tail) = block.expr {
            self.out.statements += usize::from(self.counts(tail.span));
        }
        intravisit::walk_block(self, block);
    }

    // rstyle-allow(side_effect_density): visitor accumulates results in self by design
    fn visit_arm(&mut self, arm: &'tcx rustc_hir::Arm<'tcx>) {
        // Block bodies count their own statements/tail; bare-expression arms count as one.
        let bare = !matches!(arm.body.kind, ExprKind::Block(..));
        self.out.statements += usize::from(bare && self.counts(arm.body.span));
        intravisit::walk_arm(self, arm);
    }
}

impl<'tcx> Walker<'tcx> {
    /// Walks children, wrapping user-written `unsafe { }` blocks.
    // rstyle-allow(mutation_per_target, side_effect_density): visitor accumulates results in self by design
    fn walk_expr_kind(&mut self, expr: &'tcx Expr<'tcx>) {
        let user_unsafe = matches!(
            expr.kind,
            ExprKind::Block(block, _)
                if block.rules == BlockCheckMode::UnsafeBlock(UnsafeSource::UserProvided)
        );
        if user_unsafe {
            self.effect(EffectKind::Unsafe, "unsafe block".into(), expr.span);
            self.unsafe_depth += 1;
        }
        intravisit::walk_expr(self, expr);
        self.unsafe_depth -= usize::from(user_unsafe);
    }
}
