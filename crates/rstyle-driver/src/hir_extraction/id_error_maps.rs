//! Preserve complete immediate map_err callbacks and actual immutable capture storage.

use super::hir::Context;
use rustc_hir::Expr;

impl<'tcx> Context<'tcx> {
    pub(super) fn immediate_error_map(&self, expression: &Expr<'tcx>) -> bool {
        self.immediate_core_callback(expression, &["core::result::Result::<T, E>::map_err"])
    }
}
