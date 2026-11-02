use super::*;
impl<'a, 'tcx> Collector<'a, 'tcx> {
    pub(super) fn builtin(&self, expression: &Expr<'_>) -> bool {
        self.ctx
            .typeck
            .type_dependent_def_id(expression.hir_id)
            .is_none()
    }

    pub(super) fn primitive(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        self.primitive_handler(expression)(self, expression, next)
    }

    fn primitive_handler(&self, expression: &Expr<'_>) -> super::control::Handler<'a, 'tcx> {
        match expression.kind {
            ExprKind::Binary(operator, _, _)
                if matches!(operator.node, hir::BinOpKind::And | hir::BinOpKind::Or) =>
            {
                Self::short_circuit
            }
            ExprKind::Binary(..) | ExprKind::Index(..) => Self::binary,
            ExprKind::Unary(..)
            | ExprKind::Cast(..)
            | ExprKind::Type(..)
            | ExprKind::Use(..)
            | ExprKind::Field(..)
            | ExprKind::AddrOf(..)
            | ExprKind::Repeat(..) => Self::operand,
            ExprKind::Tup(_) | ExprKind::Array(_) => Self::aggregate,
            ExprKind::Struct(..) => Self::structure,
            ExprKind::Let(_) => Self::let_expression,
            ExprKind::Path(_) => Self::path,
            ExprKind::Lit(_) | ExprKind::OffsetOf(..) => Self::skip,
            _ => Self::unmodeled,
        }
    }

    fn binary(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        let (left, right) = match expression.kind {
            ExprKind::Binary(_, left, right) | ExprKind::Index(left, right, _) => (left, right),
            _ => unreachable!(),
        };
        let right = self.expr(right, next);
        self.expr(left, right)
    }

    pub(super) fn operand(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        let inner = match expression.kind {
            ExprKind::Unary(_, inner)
            | ExprKind::Cast(inner, _)
            | ExprKind::DropTemps(inner)
            | ExprKind::Type(inner, _)
            | ExprKind::Use(inner, _)
            | ExprKind::Field(inner, _)
            | ExprKind::AddrOf(_, _, inner)
            | ExprKind::Repeat(inner, _) => inner,
            _ => unreachable!(),
        };
        self.expr(inner, next)
    }

    // A local closure value used other than by direct call could run unboundedly elsewhere.
    fn path(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        let ExprKind::Path(ref path) = expression.kind else {
            unreachable!()
        };
        match self.ctx.typeck.qpath_res(path, expression.hir_id) {
            Res::Local(id) if self.closures.contains_key(&id) => self.unsupported(
                expression.span,
                next,
                "Local closure escapes direct invocation",
            ),
            _ => next,
        }
    }

    fn skip(&mut self, _expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        next
    }

    fn unmodeled(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        self.unsupported(
            expression.span,
            next,
            "HIR expression has unmodeled control, alias, cleanup, or operator semantics",
        )
    }

    fn aggregate(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        let (ExprKind::Tup(items) | ExprKind::Array(items)) = expression.kind else {
            unreachable!()
        };
        self.arguments(items, next)
    }

    fn structure(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        let ExprKind::Struct(_, fields, tail) = expression.kind else {
            unreachable!()
        };
        let base = match tail {
            hir::StructTailExpr::Base(base) => self.expr(base, next),
            _ => next,
        };
        fields
            .iter()
            .rev()
            .fold(base, |entry, field| self.expr(field.expr, entry))
    }

    fn let_expression(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        let ExprKind::Let(binding) = expression.kind else {
            unreachable!()
        };
        self.expr(binding.init, next)
    }
}
