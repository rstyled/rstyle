//! Safe-Rust private-local frame: unrelated calls cannot write unescaped scalars.
use super::extent::{self, Place};
use super::*;
use rustc_hir::intravisit::{self, Visitor};
use rustc_middle::ty::adjustment::{Adjust, AutoBorrow, AutoBorrowMutability};

pub(super) struct Frame {
    pub extents: Vec<Place>,
    pub reason: Option<String>,
    pub exhausted: bool,
}
struct Inspect<'a, 'tcx> {
    ctx: &'a Context<'tcx>,
    stable: HashMap<HirId, bool>,
    candidates: Vec<&'tcx Expr<'tcx>>,
    selected: Vec<Place>,
    reason: Option<String>,
    work: usize,
    depth: usize,
    loops: usize,
    validating: bool,
    exhausted: bool,
}

pub(super) fn inspect<'tcx>(ctx: &Context<'tcx>, body: &'tcx hir::Body<'tcx>) -> Frame {
    let mut check = Inspect::new(ctx, body.value.hir_id.owner.def_id.to_def_id());
    check.discover(body);
    check.validate(body.value);

    Frame {
        extents: check.selected,
        reason: check.reason,
        exhausted: check.exhausted,
    }
}

impl<'a, 'tcx> Inspect<'a, 'tcx> {
    fn new(ctx: &'a Context<'tcx>, owner: hir::def_id::DefId) -> Self {
        Self {
            ctx,
            stable: HashMap::new(),
            candidates: Vec::new(),
            selected: Vec::new(),
            reason: forbidden_signature(ctx, owner)
                .then(|| "Unsafe or foreign owner signature is unmodeled".into()),
            work: 0,
            depth: 0,
            loops: 0,
            validating: false,
            exhausted: false,
        }
    }

    fn discover(&mut self, body: &'tcx hir::Body<'tcx>) {
        for parameter in body.params {
            self.parameter(parameter);
        }
        self.visit_expr(body.value);
        if !self.exhausted {
            self.select_extents();
        }
    }

    fn validate(&mut self, value: &'tcx Expr<'tcx>) {
        self.validating = true;
        if !self.exhausted {
            self.visit_expr(value);
        }
    }

    fn parameter(&mut self, parameter: &'tcx hir::Param<'tcx>) {
        if let PatKind::Binding(_, id, _, None) = parameter.pat.kind {
            self.stable.insert(id, true);
        }
        self.visit_pat(parameter.pat);
    }

    fn select_extents(&mut self) {
        let mut keys = std::collections::HashSet::new();
        for expression in &self.candidates {
            if let Some(place) = extent::place(self.ctx, expression) {
                if self.stable.get(&place.root) != Some(&true) {
                    self.reason = Some(
                        "Extent source is not a stable formal or immutable local outside loops"
                            .into(),
                    );
                }
                if keys.insert(place.key()) {
                    self.selected.push(place);
                }
                if self.selected.len() > MAX_VARIABLES {
                    self.exhausted = true;
                    return;
                }
            }
        }
    }

    fn enter(&mut self) -> bool {
        if self.exhausted {
            return false;
        }
        self.work += 1;
        if self.work > MAX_NODES * 4 || self.depth >= 256 {
            self.exhausted = true;
            return false;
        }

        self.depth += 1;
        true
    }

    fn expression(&mut self, expression: &'tcx Expr<'tcx>) {
        let reason = forbidden(self.ctx, expression)
            .then_some("Unsafe, foreign, closure, assembly or coroutine semantics are unmodeled");
        if let Some(reason) = reason {
            self.reason = Some(reason.into());
        }
        let observe: fn(&mut Self, &'tcx Expr<'tcx>) = if self.validating {
            Self::check_borrow
        } else {
            Self::record_candidate
        };
        observe(self, expression);
    }

    fn record_candidate(&mut self, expression: &'tcx Expr<'tcx>) {
        if let Some(receiver) = candidate(self.ctx, expression) {
            self.candidates.push(receiver);
        }
    }

    fn enter_roster(&mut self, width: usize) -> bool {
        if !self.enter() {
            return false;
        }
        if width > MAX_NODES * 4 {
            self.exhausted = true;
            return false;
        }
        true
    }

    fn walk_expression(&mut self, expression: &'tcx Expr<'tcx>) {
        let looping = usize::from(matches!(expression.kind, ExprKind::Loop(..)));
        self.loops += looping;
        if !matches!(expression.kind, ExprKind::Closure(_)) {
            intravisit::walk_expr(self, expression);
        }
        self.leave_expression(looping);
    }

    fn leave_expression(&mut self, looping: usize) {
        self.loops -= looping;
        self.depth -= 1;
    }

    fn record_local(&mut self, local: &hir::LetStmt<'tcx>) {
        if let PatKind::Binding(
            hir::BindingMode(hir::ByRef::No, hir::Mutability::Not),
            id,
            _,
            None,
        ) = local.pat.kind
        {
            self.stable
                .insert(id, self.loops == 0 && local.init.is_some());
        }
    }

    fn walk_local(&mut self, local: &'tcx hir::LetStmt<'tcx>) {
        intravisit::walk_local(self, local);
        self.depth -= 1;
    }

    fn check_pattern(&mut self, pattern: &hir::Pat<'tcx>) {
        if let Some(reason) = self.pattern_problem(pattern) {
            self.reason = Some(reason.into());
        }
    }

    fn pattern_problem(&self, pattern: &hir::Pat<'tcx>) -> Option<&'static str> {
        if matches!(pattern.kind, PatKind::Guard(..) | PatKind::Deref(_)) {
            return Some("Embedded pattern guards and dereference-pattern effects are unmodeled");
        }
        if self
            .ctx
            .typeck
            .pat_adjustments()
            .get(pattern.hir_id)
            .is_some_and(|adjustments| {
                adjustments.iter().any(|adjustment| {
                    adjustment.kind != rustc_middle::ty::adjustment::PatAdjust::BuiltinDeref
                })
            })
        {
            return Some("Implicit dereference-pattern effects are unmodeled");
        }

        let mutable = self
            .ctx
            .typeck
            .pat_binding_modes()
            .get(pattern.hir_id)
            .is_some_and(|mode| matches!(mode.0, hir::ByRef::Yes(_, hir::Mutability::Mut)));
        if mutable {
            return Some("Mutable reference pattern can expose a numeric norm");
        }

        if matches!(self.ctx.typeck.pat_ty(pattern).kind(),ty::Ref(_,inner,hir::Mutability::Mut) if uint_reference(*inner))
        {
            return Some("Mutable numeric-reference binding is unmodeled");
        }
        None
    }

    fn walk_pattern(&mut self, pattern: &'tcx hir::Pat<'tcx>) {
        intravisit::walk_pat(self, pattern);
        self.depth -= 1;
    }

    fn check_borrow(&mut self, expression: &Expr<'tcx>) {
        let explicit = matches!(
            expression.kind,
            ExprKind::AddrOf(_, hir::Mutability::Mut, _)
                | ExprKind::AddrOf(hir::BorrowKind::Raw, _, _)
        );
        let adjusted = self
            .ctx
            .typeck
            .expr_adjustments(expression)
            .iter()
            .any(|adjustment| {
                matches!(
                    adjustment.kind,
                    Adjust::Borrow(
                        AutoBorrow::Ref(AutoBorrowMutability::Mut { .. })
                            | AutoBorrow::RawPtr(_)
                            | AutoBorrow::Pin(_)
                    ) | Adjust::GenericReborrow(hir::Mutability::Mut)
                )
            });
        let operand = match expression.kind {
            ExprKind::AddrOf(_, _, value) => value,
            _ => expression,
        };
        if (explicit || adjusted) && self.sensitive(operand) {
            self.reason = Some(
                "Mutable or raw borrow can expose a numeric norm or selected extent metadata"
                    .into(),
            );
        }
    }

    fn sensitive(&self, expression: &Expr<'tcx>) -> bool {
        if uint_reference(self.ctx.typeck.expr_ty(expression)) {
            return true;
        }
        borrow_place(self.ctx, expression).is_some_and(|place| {
            self.selected
                .iter()
                .any(|selected| selected.overlaps(&place))
        })
    }
}

impl<'tcx> Visitor<'tcx> for Inspect<'_, 'tcx> {
    fn visit_expr(&mut self, expression: &'tcx Expr<'tcx>) {
        if !self.enter_roster(expression_width(expression)) {
            return;
        }
        self.expression(expression);
        self.walk_expression(expression);
    }

    fn visit_block(&mut self, block: &'tcx hir::Block<'tcx>) {
        if !self.enter_roster(block.stmts.len()) {
            return;
        }
        intravisit::walk_block(self, block);
        self.depth -= 1;
    }

    fn visit_ty(&mut self, _: &'tcx hir::Ty<'tcx, hir::AmbigArg>) {}

    fn visit_local(&mut self, local: &'tcx hir::LetStmt<'tcx>) {
        if !self.enter() {
            return;
        }
        self.record_local(local);
        self.walk_local(local);
    }

    fn visit_pat(&mut self, pattern: &'tcx hir::Pat<'tcx>) {
        if !self.enter_roster(pattern_width(pattern)) {
            return;
        }
        self.check_pattern(pattern);
        self.walk_pattern(pattern);
    }
}

fn uint_reference(mut ty: Ty<'_>) -> bool {
    while let ty::Ref(_, inner, _) = ty.kind() {
        ty = *inner;
    }
    matches!(ty.kind(), ty::Uint(_))
}

fn candidate<'tcx>(ctx: &Context<'tcx>, expression: &Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    if let Some(receiver) = extent::extent_receiver(ctx, expression) {
        return Some(receiver);
    }
    let parts = super::super::for_loops::parts(ctx, expression)?;
    extent::iteration_receiver(ctx, parts.head)
}

fn borrow_place(ctx: &Context<'_>, expression: &Expr<'_>) -> Option<Place> {
    borrow_place_at(ctx, expression, 256)
}

fn borrow_place_at(ctx: &Context<'_>, expression: &Expr<'_>, remaining: usize) -> Option<Place> {
    if remaining == 0 {
        return None;
    }
    match expression.kind {
        ExprKind::Path(ref path) => match ctx.typeck.qpath_res(path, expression.hir_id) {
            Res::Local(root) => Some(Place {
                root,
                fields: Vec::new(),
                label: String::new(),
            }),
            _ => None,
        },
        ExprKind::Field(base, _) => {
            let mut place = borrow_place_at(ctx, base, remaining - 1)?;
            place
                .fields
                .push(ctx.typeck.field_index(expression.hir_id).as_usize());
            Some(place)
        }
        ExprKind::Unary(hir::UnOp::Deref, base)
        | ExprKind::AddrOf(_, _, base)
        | ExprKind::Index(base, _, _) => borrow_place_at(ctx, base, remaining - 1),
        _ => None,
    }
}

fn forbidden(ctx: &Context<'_>, expression: &Expr<'_>) -> bool {
    match expression.kind {
        ExprKind::Closure(_)
        | ExprKind::InlineAsm(_)
        | ExprKind::Yield(..)
        | ExprKind::UnsafeBinderCast(..) => true,
        ExprKind::Block(block, _) => matches!(block.rules, hir::BlockCheckMode::UnsafeBlock(_)),
        ExprKind::Call(callee, _) => foreign_call(ctx, callee),
        ExprKind::MethodCall(..) => ctx
            .typeck
            .type_dependent_def_id(expression.hir_id)
            .is_some_and(|id| forbidden_signature(ctx, id)),
        _ => false,
    }
}

fn foreign_call(ctx: &Context<'_>, expression: &Expr<'_>) -> bool {
    let ty = ctx.typeck.expr_ty(expression);
    match ty.kind() {
        ty::FnDef(id, _) => {
            let signature = ctx.tcx.fn_sig(*id).instantiate_identity().skip_binder();
            signature.safety().is_unsafe()
                || signature.abi() != rustc_abi::ExternAbi::Rust
                || ctx.tcx.def_kind(ctx.tcx.parent(*id)) == hir::def::DefKind::ForeignMod
        }
        ty::FnPtr(_, header) => {
            header.safety().is_unsafe() || header.abi() != rustc_abi::ExternAbi::Rust
        }
        _ => false,
    }
}

fn forbidden_signature(ctx: &Context<'_>, id: hir::def_id::DefId) -> bool {
    let signature = ctx.tcx.fn_sig(id).instantiate_identity().skip_binder();
    signature.safety().is_unsafe() || signature.abi() != rustc_abi::ExternAbi::Rust
}

fn expression_width(expression: &Expr<'_>) -> usize {
    match expression.kind {
        ExprKind::Call(_, items)
        | ExprKind::MethodCall(_, _, items, _)
        | ExprKind::Tup(items)
        | ExprKind::Array(items) => items.len(),
        ExprKind::Match(_, arms, _) => arms.len(),
        ExprKind::Struct(_, fields, _) => fields.len(),
        _ => 0,
    }
}

fn pattern_width(pattern: &hir::Pat<'_>) -> usize {
    match pattern.kind {
        PatKind::Tuple(items, _) | PatKind::TupleStruct(_, items, _) | PatKind::Or(items) => {
            items.len()
        }
        PatKind::Struct(_, fields, _) => fields.len(),
        PatKind::Slice(before, _, after) => before.len().saturating_add(after.len()),
        _ => 0,
    }
}
