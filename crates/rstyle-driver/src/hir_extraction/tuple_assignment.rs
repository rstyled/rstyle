//! Recover only the compiler's scalar-local tuple assignment desugaring.
use super::hir::Context;
use rstyle_pdg::HirNode;
use rustc_hir::{self as hir, Expr, ExprKind, PatKind, StmtKind, def::Res};
use rustc_middle::ty;

impl<'tcx> Context<'tcx> {
    pub(super) fn tuple_assignment(&self, expression: &Expr<'tcx>) -> Option<HirNode> {
        let parts = parts(expression)?;
        if !self.profile.supports("rust.scalar-tuple-assignment") {
            return None;
        }
        let targets = parts
            .patterns
            .iter()
            .zip(parts.assignments)
            .map(|(pattern, statement)| self.tuple_target(pattern, statement))
            .collect::<Option<Vec<_>>>()?;
        let local = parts.local;
        let rhs = parts.rhs;

        let mut left = self.node(local.pat.hir_id, local.pat.span, "expr_tup");
        left.ty = Some(super::hir::resolved(
            self.typeck.pat_ty(local.pat),
            String::new(),
        ));
        left.children = targets;
        let mut node = self.node(expression.hir_id, expression.span, "expr_tuple_assign");
        node.ty = Some(super::hir::resolved(
            self.typeck.expr_ty(expression),
            String::new(),
        ));
        node.children = vec![left, self.expression(rhs)];
        Some(node)
    }
    fn tuple_target(
        &self,
        pattern: &hir::Pat<'tcx>,
        statement: &hir::Stmt<'tcx>,
    ) -> Option<HirNode> {
        let PatKind::Binding(_, temporary, _, None) = pattern.kind else {
            return None;
        };
        let StmtKind::Expr(assignment) = statement.kind else {
            return None;
        };
        let ExprKind::Assign(target, value, _) = assignment.kind else {
            return None;
        };
        let ExprKind::Path(ref path) = value.kind else {
            return None;
        };
        if self.typeck.qpath_res(path, value.hir_id) != Res::Local(temporary)
            || !matches!(target.kind, ExprKind::Path(_))
            || !matches!(
                self.typeck.expr_ty(target).kind(),
                ty::Bool | ty::Int(_) | ty::Uint(_)
            )
        {
            return None;
        }

        Some(self.expression(target))
    }
}

struct Parts<'tcx> {
    local: &'tcx hir::LetStmt<'tcx>,
    patterns: &'tcx [hir::Pat<'tcx>],
    assignments: &'tcx [hir::Stmt<'tcx>],
    rhs: &'tcx Expr<'tcx>,
}

fn parts<'tcx>(expression: &Expr<'tcx>) -> Option<Parts<'tcx>> {
    let ExprKind::Block(block, None) = expression.kind else {
        return None;
    };
    let [first, assignments @ ..] = block.stmts else {
        return None;
    };
    let StmtKind::Let(local) = first.kind else {
        return None;
    };
    if !matches!(local.source, hir::LocalSource::AssignDesugar) || block.expr.is_some() {
        return None;
    }
    let PatKind::Tuple(patterns, rest) = local.pat.kind else {
        return None;
    };
    let rhs = local.init?;
    let ExprKind::Tup(values) = rhs.kind else {
        return None;
    };

    if rest.as_opt_usize().is_some()
        || patterns.is_empty()
        || patterns.len() != assignments.len()
        || patterns.len() != values.len()
    {
        return None;
    }
    Some(Parts {
        local,
        patterns,
        assignments,
        rhs,
    })
}
