//! Nondeterministic branches retain all possible evaluation paths and early exits.
use super::*;
impl<'tcx> Collector<'_, 'tcx> {
    pub(super) fn short_circuit(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        let ExprKind::Binary(_, left, right) = expression.kind else {
            unreachable!()
        };
        let right = self.expr(right, next);
        let branch = self.choice((right, next), expression.span);
        self.expr(left, branch)
    }

    pub(super) fn matching(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        let ExprKind::Match(scrutinee, arms, _) = expression.kind else {
            unreachable!()
        };
        if arms.iter().any(guarded_or) {
            return self.unsupported(
                expression.span,
                next,
                "Guarded or-pattern reevaluation is unmodeled",
            );
        }
        let branches = self.match_arms(arms, next);
        self.expr(scrutinee, branches)
    }

    fn match_arms(&mut self, arms: &'tcx [hir::Arm<'tcx>], next: u32) -> u32 {
        let mut fallback = next;
        for arm in arms.iter().rev() {
            fallback = self.match_arm(arm, (next, fallback));
        }
        fallback
    }

    fn match_arm(&mut self, arm: &'tcx hir::Arm<'tcx>, destinations: (u32, u32)) -> u32 {
        let body = self.expr(arm.body, destinations.0);
        let selected = self.arm_guard(arm.guard, (body, destinations.1), arm.span);
        self.choice((selected, destinations.1), arm.span)
    }

    fn arm_guard(
        &mut self,
        guard: Option<&'tcx Expr<'tcx>>,
        branches: (u32, u32),
        span: rustc_span::Span,
    ) -> u32 {
        match guard {
            Some(guard) => {
                let branch = self.choice(branches, span);
                self.expr(guard, branch)
            }
            None => branches.0,
        }
    }

    pub(super) fn choice(&mut self, branches: (u32, u32), span: rustc_span::Span) -> u32 {
        self.node(
            Action::Nop,
            Flow::Branch {
                condition: Condition::Unknown,
                then_node: branches.0,
                else_node: branches.1,
            },
            span,
        )
    }
}

fn guarded_or(arm: &hir::Arm<'_>) -> bool {
    if arm.guard.is_none() {
        return false;
    }
    let mut found = false;
    arm.pat
        .walk_always(|pattern| found |= matches!(pattern.kind, PatKind::Or(_)));
    found
}
