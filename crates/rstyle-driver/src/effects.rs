//! One HIR walk per function: statement count, resolved calls, and effects
//! that need no config (`unsafe`, `static mut`, indirect calls).

use crate::ids;
use rstyle_core::facts::{CallEvent, Edge, EffectEvent, EffectKind};
use rstyle_core::paths::strip_generics;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{BlockCheckMode, Body, Expr, ExprKind, QPath, Stmt, UnsafeSource};
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
}

pub fn collect<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: LocalDefId,
    body: &'tcx Body<'tcx>,
    root: &Path,
) -> BodyFacts {
    let mut walker = Walker {
        tcx,
        typeck: tcx.typeck(def_id),
        root: root.to_path_buf(),
        out: BodyFacts::default(),
        generated_depth: 0,
        unsafe_depth: 0,
    };
    walker.visit_body(body);
    walker.out
}

struct Walker<'tcx> {
    tcx: TyCtxt<'tcx>,
    typeck: &'tcx TypeckResults<'tcx>,
    root: PathBuf,
    out: BodyFacts,
    /// Nonzero while inside macro/desugaring-generated code. Generated statements
    /// are counted once (the outermost), user-written ones always.
    generated_depth: usize,
    /// Nonzero while inside a user-written `unsafe` block.
    unsafe_depth: usize,
}

impl<'tcx> Walker<'tcx> {
    fn line(&self, span: Span) -> usize {
        self.tcx
            .sess
            .source_map()
            .lookup_char_pos(span.source_callsite().lo())
            .line
    }

    fn effect(&mut self, kind: EffectKind, detail: String, span: Span) {
        let line = self.line(span);
        self.out.effects.push(EffectEvent { kind, detail, line });
    }

    fn counts(&self, span: Span) -> bool {
        !span.from_expansion() || self.generated_depth == 0
    }

    fn call(&mut self, def_id: DefId, dynamic: bool, expr: &Expr<'tcx>) {
        if !is_fn(self.tcx, def_id) {
            return;
        }
        self.edge(def_id, dynamic, expr);
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
    fn edge(&mut self, def_id: DefId, dynamic: bool, expr: &Expr<'tcx>) {
        if !ids::in_workspace(self.tcx, def_id, &self.root) {
            return;
        }
        let line = self.line(expr.span);
        let callee = ids::def_id_str(self.tcx, def_id);
        self.out.edges.push(Edge {
            callee,
            line,
            dynamic,
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

    fn on_call(&mut self, expr: &Expr<'tcx>, callee: &Expr<'tcx>) {
        match self.typeck.expr_ty(callee).kind() {
            ty::FnDef(def_id, _) => self.call(*def_id, false, expr),
            ty::Closure(..) | ty::CoroutineClosure(..) => {}
            ty::FnPtr(..) => self.indirect("<fn pointer>", expr),
            _ => self.indirect("<indirect callable>", expr),
        }
    }

    fn on_method(&mut self, expr: &Expr<'tcx>, receiver: &Expr<'tcx>) {
        let Some(def_id) = self.typeck.type_dependent_def_id(expr.hir_id) else {
            return;
        };
        let receiver_ty = self.typeck.expr_ty_adjusted(receiver).peel_refs();
        let dynamic = matches!(receiver_ty.kind(), ty::Dynamic(..));
        self.call(def_id, dynamic, expr);
    }

    fn on_path(&mut self, expr: &Expr<'tcx>, qpath: &QPath<'tcx>) {
        if let Res::Def(DefKind::Static { mutability, .. }, def_id) =
            self.typeck.qpath_res(qpath, expr.hir_id)
            && mutability.is_mut()
        {
            let name = strip_generics(&self.tcx.def_path_str(def_id));
            self.effect(EffectKind::Global, format!("static mut {name}"), expr.span);
        }
    }
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
            ExprKind::Call(callee, _) => self.on_call(expr, callee),
            ExprKind::MethodCall(_, receiver, ..) => self.on_method(expr, receiver),
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
