//! Compiler-resolved structural correspondence for Rust's for-loop desugaring.
use super::hir::Context;
use rustc_hir::{self as hir, Expr, ExprKind, PatKind, StmtKind, def::Res};

pub(super) struct Parts<'tcx> {
    pub head: &'tcx Expr<'tcx>,
    pub pattern: &'tcx hir::Pat<'tcx>,
    pub body: &'tcx Expr<'tcx>,
    pub iteration: &'tcx Expr<'tcx>,
}

pub(super) fn parts<'tcx>(ctx: &Context<'tcx>, expression: &Expr<'tcx>) -> Option<Parts<'tcx>> {
    let ExprKind::Match(scrutinee, [outer], hir::MatchSource::ForLoopDesugar) = expression.kind
    else {
        return None;
    };
    let head = language_argument(ctx, scrutinee, ctx.tcx.lang_items().into_iter_fn()?)?;
    let iteration = peeled(outer.body);
    let ExprKind::Loop(block, None, hir::LoopSource::ForLoop, _) = iteration.kind else {
        return None;
    };

    let ExprKind::Match(next, [none, some], _) = peeled(last_expression(block)?).kind else {
        return None;
    };
    language_argument(ctx, next, ctx.tcx.lang_items().next_fn()?)?;
    if outer.guard.is_some() || none.guard.is_some() || some.guard.is_some() {
        return None;
    }

    let pattern = match some.pat.kind {
        PatKind::TupleStruct(_, [pattern], _) => pattern,
        PatKind::Struct(_, [field], _) => field.pat,
        _ => return None,
    };
    if !matches!(some.body.kind, ExprKind::Block(_, None)) {
        return None;
    }

    Some(Parts {
        head,
        pattern,
        body: some.body,
        iteration,
    })
}

fn language_argument<'tcx>(
    ctx: &Context<'tcx>,
    expression: &'tcx Expr<'tcx>,
    expected: hir::def_id::DefId,
) -> Option<&'tcx Expr<'tcx>> {
    let ExprKind::Call(callee, [argument]) = expression.kind else {
        return None;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return None;
    };
    (ctx.typeck.qpath_res(path, callee.hir_id) == Res::Def(hir::def::DefKind::AssocFn, expected))
        .then_some(argument)
}
pub(super) fn peeled<'tcx>(mut expression: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    loop {
        expression = match expression.kind {
            ExprKind::DropTemps(inner) => inner,
            ExprKind::Block(block, _) if block.stmts.is_empty() => match block.expr {
                Some(inner) => inner,
                None => return expression,
            },
            _ => return expression,
        };
    }
}

fn last_expression<'tcx>(block: &'tcx hir::Block<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    block.expr.or_else(|| match block.stmts.last()?.kind {
        StmtKind::Expr(expression) | StmtKind::Semi(expression) => Some(expression),
        _ => None,
    })
}
