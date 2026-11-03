pub(super) use super::super::for_loops::peeled;
use super::*;
impl Collector<'_, '_> {
    pub(super) fn bits(&self, ty: Ty<'_>) -> Option<u8> {
        match ty.kind() {
            ty::Uint(kind) => Some(match kind {
                ty::UintTy::U8 => 8,
                ty::UintTy::U16 => 16,
                ty::UintTy::U32 => 32,
                ty::UintTy::U64 => 64,
                ty::UintTy::Usize => self.ctx.tcx.data_layout.pointer_size().bits() as u8,
                _ => return None,
            }),
            _ => None,
        }
    }
    pub(super) fn local_id(&self, expression: &Expr<'_>) -> Option<u32> {
        let ExprKind::Path(ref path) = expression.kind else {
            return None;
        };
        let Res::Local(id) = self.ctx.typeck.qpath_res(path, expression.hir_id) else {
            return None;
        };
        self.locals.get(&id).copied()
    }
    pub(super) fn value(&self, expression: &Expr<'_>) -> Value {
        if let Some(variable) = self.local_id(expression) {
            return Value::Variable(variable);
        }
        if let Some(value) = self.extent_value(expression) {
            return value;
        }
        match expression.kind {
            ExprKind::Lit(literal) => match literal.node {
                rustc_ast::LitKind::Int(value, _) => {
                    u64::try_from(value.get()).map_or(Value::Unknown, Value::Const)
                }
                // A byte literal is the integer it names.
                rustc_ast::LitKind::Byte(value) => Value::Const(u64::from(value)),
                _ => Value::Unknown,
            },
            ExprKind::DropTemps(inner) => self.value(inner),
            _ => Value::Unknown,
        }
    }
    pub(super) fn condition(&self, expression: &Expr<'_>) -> Condition {
        let expression = peeled(expression);
        if let ExprKind::Binary(operator, left, right) = expression.kind
            && operator.node == hir::BinOpKind::Lt
            && self
                .ctx
                .typeck
                .type_dependent_def_id(expression.hir_id)
                .is_none()
            && self.bits(self.ctx.typeck.expr_ty(left)).is_some()
            && self.ctx.typeck.expr_ty(left) == self.ctx.typeck.expr_ty(right)
            && let Some(variable) = self.local_id(left)
        {
            return Condition::Less {
                variable,
                limit: self.value(right),
            };
        }
        Condition::Unknown
    }
}
