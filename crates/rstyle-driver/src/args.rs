//! Provenance of call-site arguments, relative to the calling function's parameters.

use rstyle_core::facts::ArgSource;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{Expr, ExprKind, HirId, QPath, UnOp};
use rustc_middle::ty::TypeckResults;
use std::collections::HashMap;

/// Binding id of each parameter of the function being walked -> its index (`self` excluded).
pub type ParamIndex = HashMap<HirId, usize>;

/// Strips `&`, `&mut`, `*` and temporaries: they change how a value is passed, not what it is.
fn peel<'a, 'tcx>(mut expr: &'a Expr<'tcx>) -> &'a Expr<'tcx> {
    loop {
        match expr.kind {
            ExprKind::AddrOf(_, _, inner)
            | ExprKind::Unary(UnOp::Deref, inner)
            | ExprKind::DropTemps(inner) => expr = inner,
            _ => return expr,
        }
    }
}

fn local_id(typeck: &TypeckResults<'_>, expr: &Expr<'_>) -> Option<HirId> {
    let ExprKind::Path(ref qpath) = expr.kind else {
        return None;
    };
    match typeck.qpath_res(qpath, expr.hir_id) {
        Res::Local(id) => Some(id),
        _ => None,
    }
}

fn is_constant(typeck: &TypeckResults<'_>, qpath: &QPath<'_>, expr: &Expr<'_>) -> bool {
    matches!(
        typeck.qpath_res(qpath, expr.hir_id),
        Res::Def(
            DefKind::Const
                | DefKind::AssocConst
                | DefKind::ConstParam
                | DefKind::Static { .. }
                | DefKind::Ctor(..),
            _
        )
    )
}

/// True for `a.b.c` chains whose root is a local binding.
fn local_field_chain(typeck: &TypeckResults<'_>, expr: &Expr<'_>) -> bool {
    match peel(expr).kind {
        ExprKind::Field(base, _) => {
            local_field_chain(typeck, base) || local_id(typeck, peel(base)).is_some()
        }
        _ => false,
    }
}

pub fn classify(typeck: &TypeckResults<'_>, params: &ParamIndex, arg: &Expr<'_>) -> ArgSource {
    let arg = peel(arg);
    if let Some(index) = local_id(typeck, arg).and_then(|id| params.get(&id)) {
        return ArgSource::Param(*index);
    }
    match arg.kind {
        ExprKind::Lit(_) => ArgSource::Const,
        ExprKind::Path(ref qpath) if is_constant(typeck, qpath, arg) => ArgSource::Const,
        ExprKind::Field(..) if local_field_chain(typeck, arg) => ArgSource::Field,
        _ => ArgSource::Other,
    }
}
