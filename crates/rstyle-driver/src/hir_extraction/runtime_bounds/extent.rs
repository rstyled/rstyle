//! Immutable standard-library extent values, identified by resolved types and by
//! the extent roles their methods have in the library contracts.
use super::*;
use contracts::{Contracts, ExtentRole};
use rustc_middle::ty::adjustment::{Adjust, AutoBorrow, AutoBorrowMutability, DerefAdjustKind};
use rustc_span::sym;

#[derive(Clone)]
pub(super) struct Place {
    pub root: HirId,
    pub fields: Vec<usize>,
    /// Named field path, for composing formal-rooted lengths across calls.
    pub path: Vec<Field>,
    pub label: String,
}
impl Place {
    pub fn key(&self) -> String {
        format!("{}:{:?}", self.root.local_id.as_u32(), self.fields)
    }
    pub fn overlaps(&self, other: &Self) -> bool {
        self.root == other.root
            && (self.fields.starts_with(&other.fields) || other.fields.starts_with(&self.fields))
    }
}

impl Collector<'_, '_> {
    pub(super) fn extent_value(&self, expression: &Expr<'_>) -> Option<Value> {
        let receiver = extent_receiver(self.ctx, expression)?;
        let place = place(self.ctx, receiver)?;
        self.extents
            .get(&place.key())
            .map(|label| Value::Input(label.clone()))
    }
}

pub(super) fn is_vec(ctx: &Context<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(),ty::Adt(definition,_) if ctx.tcx.is_diagnostic_item(sym::Vec,definition.did()))
}

pub(super) fn shared_type(mut ty: Ty<'_>) -> Option<Ty<'_>> {
    while let ty::Ref(_, inner, mutability) = ty.kind() {
        if mutability.is_mut() {
            return None;
        }
        ty = *inner;
    }
    Some(ty)
}

pub(super) fn extent_receiver<'tcx>(
    ctx: &Context<'_>,
    expression: &Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    let ExprKind::MethodCall(_, receiver, [], _) = expression.kind else {
        return None;
    };
    has_role(ctx, expression, ExtentRole::Length).then_some(receiver)
}

/// Whether the resolved method is an inherent standard member with this extent role.
/// Trait members are excluded: their impl, not the contract, decides what they read.
pub(super) fn has_role(ctx: &Context<'_>, expression: &Expr<'_>, role: ExtentRole) -> bool {
    let Some(method) = ctx.typeck.type_dependent_def_id(expression.hir_id) else {
        return false;
    };
    let inherent = ctx.tcx.inherent_impl_of_assoc(method).is_some()
        && super::sysroot::sysroot(ctx.tcx, method.krate);
    let key = inherent.then(|| super::sysroot::operation(ctx.tcx, method));
    key.flatten()
        .and_then(|key| Contracts::embedded().extent(&key))
        .is_some_and(|extent| extent.role == role)
}

pub(super) fn place(ctx: &Context<'_>, expression: &Expr<'_>) -> Option<Place> {
    place_at(ctx, expression, 256)
}

fn place_at(ctx: &Context<'_>, expression: &Expr<'_>, remaining: usize) -> Option<Place> {
    if remaining == 0 {
        return None;
    }
    if !standard_adjustments(ctx, expression) {
        return None;
    }
    match expression.kind {
        ExprKind::Path(ref path) => {
            let Res::Local(root) = ctx.typeck.qpath_res(path, expression.hir_id) else {
                return None;
            };
            shared_type(ctx.typeck.expr_ty(expression))?;
            Some(Place {
                root,
                fields: Vec::new(),
                path: Vec::new(),
                label: format!(
                    "len({})#{}",
                    compact(&ctx.snippet(expression.span)),
                    root.local_id.as_u32()
                ),
            })
        }
        ExprKind::Field(base, name) => {
            let mut place = field_place(ctx, expression, base, remaining - 1)?;
            let index = ctx.typeck.field_index(expression.hir_id).as_u32();
            place.path.push(Field {
                index,
                name: name.name.to_string(),
            });
            Some(place)
        }
        ExprKind::Unary(hir::UnOp::Deref, base)
            if matches!(
                ctx.typeck.expr_ty(base).kind(),
                ty::Ref(_, _, hir::Mutability::Not)
            ) =>
        {
            place_at(ctx, base, remaining - 1)
        }
        ExprKind::AddrOf(hir::BorrowKind::Ref, hir::Mutability::Not, base) => {
            place_at(ctx, base, remaining - 1)
        }
        _ => None,
    }
}

fn field_place(
    ctx: &Context<'_>,
    expression: &Expr<'_>,
    base: &Expr<'_>,
    remaining: usize,
) -> Option<Place> {
    let mut place = place_at(ctx, base, remaining)?;
    let root_ty = ctx.typeck.node_type(place.root);
    if !matches!(root_ty.kind(), ty::Ref(_, _, hir::Mutability::Not)) {
        return None;
    }

    place
        .fields
        .push(ctx.typeck.field_index(expression.hir_id).as_usize());
    place.label = format!(
        "len({})#{}",
        compact(&ctx.snippet(expression.span)),
        place.root.local_id.as_u32()
    );
    Some(place)
}

// Labels are identities; source line breaks and indentation are not part of them.
fn compact(snippet: &str) -> String {
    snippet.split_whitespace().collect()
}

pub(super) fn standard_adjustments(ctx: &Context<'_>, expression: &Expr<'_>) -> bool {
    let mut current = ctx.typeck.expr_ty(expression);
    for adjustment in ctx.typeck.expr_adjustments(expression) {
        let allowed = match adjustment.kind {
            Adjust::Deref(DerefAdjustKind::Builtin) => {
                matches!(current.kind(), ty::Ref(_, _, hir::Mutability::Not))
            }
            Adjust::Deref(DerefAdjustKind::Overloaded(ref operation)) => {
                operation.mutbl == hir::Mutability::Not
                    && is_vec(ctx, current)
                    && matches!(adjustment.target.kind(), ty::Slice(_))
            }
            Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Not)) => true,
            _ => false,
        };
        if !allowed {
            return false;
        }
        current = adjustment.target;
    }
    true
}

pub(super) fn iteration_receiver<'tcx>(
    ctx: &Context<'_>,
    head: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::MethodCall(_, receiver, [], _) = head.kind {
        return has_role(ctx, head, ExtentRole::Traversal).then_some(receiver);
    }
    extent_type(ctx, ctx.typeck.expr_ty(head)).then_some(head)
}

pub(super) fn extent_type(ctx: &Context<'_>, ty: Ty<'_>) -> bool {
    shared_type(ty).is_some_and(|ty| matches!(ty.kind(), ty::Slice(_)) || is_vec(ctx, ty))
}
