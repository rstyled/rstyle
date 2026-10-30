//! Original primitive range iteration, without generated iterator operations.
use super::*;
use rustc_span::hygiene::{DesugaringKind, ExpnKind};

struct Parts<'tcx> {
    head: &'tcx Expr<'tcx>,
    pattern: &'tcx hir::Pat<'tcx>,
    body: &'tcx Expr<'tcx>,
    iteration: &'tcx Expr<'tcx>,
}

impl<'tcx> Context<'tcx> {
    pub(super) fn original_for_loop(&self, expression: &Expr<'tcx>) -> Option<HirNode> {
        if !self.profile.is_data()
            || !matches!(
                expression.kind,
                ExprKind::Match(_, _, hir::MatchSource::ForLoopDesugar)
            )
            || !original_span(expression.span)
        {
            return None;
        }
        let parts = self.for_parts(expression)?;
        let head = self.original_range(parts.head)?;
        if !matches!(
            parts.pattern.kind,
            PatKind::Wild
                | PatKind::Binding(
                    hir::BindingMode(hir::ByRef::No, hir::Mutability::Not),
                    _,
                    _,
                    None
                )
        ) {
            return None;
        }

        Some(HirNode {
            ty: Some(resolved(self.typeck.expr_ty(expression), String::new())),
            res: Some(PathRes {
                kind: "innermost_loop".into(),
                target: Some(reference(parts.iteration.hir_id)),
                description: "Compiler-resolved original for-loop destination".into(),
            }),
            children: vec![
                head,
                self.pattern(parts.pattern),
                self.expression(parts.body),
            ],
            ..self.node(expression.hir_id, expression.span, "expr_for")
        })
    }

    fn original_range(&self, expression: &Expr<'tcx>) -> Option<HirNode> {
        if !original_span(expression.span) {
            return None;
        }
        let ExprKind::Struct(_, fields, hir::StructTailExpr::None) = expression.kind else {
            return None;
        };
        let [start, end] = fields else {
            return None;
        };
        if start.ident.name.as_str() != "start" || end.ident.name.as_str() != "end" {
            return None;
        }
        let ty::Adt(definition, arguments) = self.typeck.expr_ty(expression).kind() else {
            return None;
        };
        if Some(definition.did()) != self.tcx.lang_items().range_struct()
            || self.tcx.crate_name(definition.did().krate).as_str() != "core"
            || arguments.len() != 1
            || arguments.type_at(0) != self.tcx.types.u32
            || self.typeck.expr_ty(start.expr) != self.tcx.types.u32
            || self.typeck.expr_ty(end.expr) != self.tcx.types.u32
        {
            return None;
        }

        let mut resolution = self.library_callee(definition.did());
        resolution.kind = "resolved_u32_range_type".into();

        Some(HirNode {
            ty: Some(resolved(self.typeck.expr_ty(expression), String::new())),
            res: Some(resolution),
            children: vec![self.expression(start.expr), self.expression(end.expr)],
            ..self.node(expression.hir_id, expression.span, "expr_struct")
        })
    }

    fn for_parts(&self, expression: &Expr<'tcx>) -> Option<Parts<'tcx>> {
        let ExprKind::Match(scrutinee, [outer], hir::MatchSource::ForLoopDesugar) = expression.kind
        else {
            return None;
        };
        let head = self.language_argument(scrutinee, self.tcx.lang_items().into_iter_fn()?)?;
        let iteration = peeled(outer.body);
        let ExprKind::Loop(block, None, hir::LoopSource::ForLoop, _) = iteration.kind else {
            return None;
        };
        let ExprKind::Match(next, [none, some], _) = peeled(last_expression(block)?).kind else {
            return None;
        };
        self.language_argument(next, self.tcx.lang_items().next_fn()?)?;
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

    fn language_argument(
        &self,
        expression: &'tcx Expr<'tcx>,
        expected: hir::def_id::DefId,
    ) -> Option<&'tcx Expr<'tcx>> {
        let ExprKind::Call(callee, [argument]) = expression.kind else {
            return None;
        };
        let ExprKind::Path(ref path) = callee.kind else {
            return None;
        };
        (self.typeck.qpath_res(path, callee.hir_id)
            == Res::Def(hir::def::DefKind::AssocFn, expected))
        .then_some(argument)
    }
}

fn peeled<'tcx>(mut expression: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
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

fn original_span(mut span: Span) -> bool {
    while span.from_expansion() {
        let expansion = span.ctxt().outer_expn_data();
        if !matches!(
            expansion.kind,
            ExpnKind::Desugaring(DesugaringKind::ForLoop | DesugaringKind::RangeExpr)
        ) {
            return false;
        }
        span = expansion.call_site;
    }
    true
}
