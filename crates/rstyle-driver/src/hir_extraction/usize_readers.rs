//! Observations for immediate Result reader callbacks; no callable or effect authority.
use super::hir::Context;
use rstyle_pdg::HirNode;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{self as hir, Expr};
use rustc_middle::ty;

pub(super) const CAPABILITY: &str = "rust.usize-reader-callback-observations";

impl<'tcx> Context<'tcx> {
    pub(super) fn usize_reader_callback(&self, expression: &Expr<'tcx>) -> bool {
        self.profile.supports(CAPABILITY)
            && self.usize_reader_result()
            && self.immediate_core_callback(
                expression,
                &[
                    "core::result::Result::<T, E>::and_then",
                    "std::result::Result::<T, E>::and_then",
                    "core::result::Result::<T, E>::map",
                    "std::result::Result::<T, E>::map",
                    "core::result::Result::<T, E>::map_err",
                    "std::result::Result::<T, E>::map_err",
                ],
            )
            && self.usize_reader_return_free(expression)
    }

    fn usize_reader_return_free(&self, expression: &Expr<'tcx>) -> bool {
        let hir::ExprKind::Closure(closure) = expression.kind else {
            return false;
        };
        let mut scan = ReturnScan {
            tcx: self.tcx,
            found: false,
            remaining: rstyle_pdg::concrete_types::MAX_ENTRIES,
            depth: 0,
        };
        scan.visit_expr(self.tcx.hir_body(closure.body).value);

        !scan.found
    }

    pub(super) fn usize_reader_result(&self) -> bool {
        let ty::Adt(definition, arguments) = self.result.kind() else {
            return false;
        };
        if !self
            .tcx
            .is_diagnostic_item(rustc_span::sym::Result, definition.did())
        {
            return false;
        }

        matches!(arguments.type_at(0).kind(), ty::Tuple(fields)
            if fields.len() == 2 && fields[0] == self.tcx.types.usize
                && fields[1] == self.tcx.types.usize)
    }

    pub(super) fn usize_reader_children(&self, expression: &Expr<'tcx>) -> Option<Vec<HirNode>> {
        self.closure_children(expression, |pattern| self.local_pattern(pattern))
    }

}

struct ReturnScan<'tcx> {
    tcx: rustc_middle::ty::TyCtxt<'tcx>,
    found: bool,
    remaining: usize,
    depth: usize,
}

impl<'tcx> Visitor<'tcx> for ReturnScan<'tcx> {
    fn visit_expr(&mut self, expression: &'tcx Expr<'tcx>) {
        if !self.enter(expression) {
            return;
        }

        self.walk(expression);
        self.depth -= 1;
    }
}

impl<'tcx> ReturnScan<'tcx> {
    fn enter(&mut self, expression: &Expr<'_>) -> bool {
        if self.found
            || self.remaining == 0
            || self.depth == rstyle_pdg::concrete_types::MAX_DEPTH
            || matches!(expression.kind, hir::ExprKind::Ret(_))
        {
            self.found = true;
            return false;
        }
        self.remaining -= 1;
        self.depth += 1;

        true
    }

    fn walk(&mut self, expression: &'tcx Expr<'tcx>) {
        match expression.kind {
            hir::ExprKind::Closure(closure) => {
                self.visit_expr(self.tcx.hir_body(closure.body).value);
            }
            _ => intravisit::walk_expr(self, expression),
        }
    }
}
